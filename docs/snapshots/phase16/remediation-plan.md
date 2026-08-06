# Phase 16 Remediation Plan

**Date**: 2026-08-05
**Audit sources**: `audit.codex.md`, `audit.deepseek-v4-flash.md`
**Commit range**: `1021842c937653de545cd335450df985f822bd06..f8aff0237221fbf7d56b58abb5dce02833344bfc`
**Verified code**: `2c48c85638000df02880db1ec881f12fdcb96f6c`
**Design specs**: `docs/opi-spec.md`, `docs/superpowers/specs/2026-07-28-phase16-pluggable-extension-command-execution-design.md`

---

## Audit cross-reference summary

The reports inspect different baselines: Codex audited the archived Phase 16
exit at `f8aff02`, while DeepSeek audited the post-remediation tree at
`2c48c85`. Consensus therefore records report overlap, not proof that a defect
still exists. Every row below was independently rechecked against `2c48c85`;
that verification status controls the plan.

With two auditors, a finding is either full consensus (2/2) or unique (1/2).
Codex's two Blockers and most of its Majors were fixed by `2b23010`; the rows
remain here so no audit finding silently disappears.

| Cluster | Theme | Auditors | Consensus | Unified severity | Verification |
|---|---|---|---|---|---|
| C01 | Same-source re-add preserves trust / failed add leaves metadata | Codex 1.1, 1.5 | Unique (1/2) | Blocker | Refuted at HEAD; fixed by `2b23010` |
| C02 | Target starts before flushed `started` | Codex 1.2 | Unique (1/2) | Blocker | Refuted at HEAD; release gate and sentinel test exist |
| C03 | Native policy grants system temp | Codex 1.3 | Unique (1/2) | Major | Refuted at HEAD; private invocation temp is used |
| C04 | Sandbox tree/cleanup failures are erased | Codex 1.4 | Unique (1/2) | Major | Refuted at HEAD in `opi-sandbox` |
| C05 | `ready` lacks identity/target binding | Codex 2.1 | Unique (1/2) | Major | Refuted at HEAD |
| C06 | Protocol bounds/closure and integration coverage | Codex 2.2; DeepSeek 4.3 | Full (2/2) | Major (Major/Minor) | Partially confirmed: most bounds fixed; `failed.message` and integration coverage remain |
| C07 | Premature EOF and simultaneous stdin-close handling | Codex 2.3; DeepSeek 2.3 | Full (2/2) | Major (Major/Info) | Partially confirmed: premature EOF fixed; simultaneous-close precedence remains informational |
| C08 | Native strings are converted lossily | Codex 2.4 | Unique (1/2) | Major | Refuted at HEAD |
| C09 | Initialize deadline/configuration is ignored | Codex 2.5 | Unique (1/2) | Major | Partially confirmed: deadline fixed; `adapter_config` still ignored |
| C10 | Cancellation bypasses host protocol state | Codex 2.6 | Unique (1/2) | Major | Confirmed |
| C11 | Terminal diagnostics are discarded | Codex 2.7 | Unique (1/2) | Minor | Refuted at HEAD |
| C12 | Production Minimal Runtime constructs extension state | Codex 3.1; DeepSeek 5.2, 5.5 | Full (2/2) | Major (Major/Minor/Info) | Confirmed; default startup reads activation state and allocates permission state before branching |
| C13 | Model routing advertises incompatible adapters | Codex 3.2 | Unique (1/2) | Major | Refuted at HEAD |
| C14 | Handshake timeout is unused | Codex 3.3 | Unique (1/2) | Major | Refuted at HEAD |
| C15 | Archives omit schema/license | Codex 4.1 | Unique (1/2) | Major | Refuted at HEAD |
| C16 | Extracted smoke omits mandatory acceptance | Codex 4.2 | Unique (1/2) | Major | Partially confirmed: argv/I/O/exit/backend paths were added; setup, empty-cwd, FS, and network sentinels remain |
| C17 | macOS launch snapshot hashes from EOF | DeepSeek 2.1 | Unique (1/2) | Major | Confirmed |
| C18 | Signal death is dropped and misreported | DeepSeek 2.2 | Unique (1/2) | Minor | Confirmed |
| C19 | Raw model backend is echoed in diagnostics | DeepSeek 3.1 | Unique (1/2) | Minor | Partially confirmed; schema callers are bounded, direct callers are not |
| C20 | Windows resume failure drops degradation | DeepSeek 3.2 | Unique (1/2) | Minor | Confirmed |
| C21 | Doctor swallows activation-store read failures | DeepSeek 3.3 | Unique (1/2) | Minor | Confirmed |
| C22 | Legacy sandbox action omits migration needles | DeepSeek 3.4 | Unique (1/2) | Minor | Confirmed |
| C23 | Unavailable adapter is always labeled store failure | DeepSeek 3.5 | Unique (1/2) | Info | Confirmed diagnostic-quality gap |
| C24 | Crate-wide unsafe prohibition is incomplete | DeepSeek 3.6 | Unique (1/2) | Info | Partially confirmed; leaf modules are guarded, FFI prevents a crate-root `forbid` |
| C25 | Feature-gated acceptance suites do not run in CI | DeepSeek 4.1 | Unique (1/2) | Major | Confirmed |
| C26 | Windows target-mismatch test trips version gate first | DeepSeek 4.2 | Unique (1/2) | Major | Confirmed |
| C27 | SDK effective contract is only type-checked | DeepSeek 4.4 | Unique (1/2) | Minor | Confirmed |
| C28 | Human CLI stdin byte flow is only structural | DeepSeek 4.5 | Unique (1/2) | Minor | Confirmed |
| C29 | Phase-exit audit accepts ignored tests | DeepSeek 4.6 | Unique (1/2) | Minor | Confirmed |
| C30 | Backend failure codes/phases lack coverage | DeepSeek 4.7 | Unique (1/2) | Minor | Confirmed; cleanup phase semantics are also inconsistent |
| C31 | `ProtocolId` accepts empty strings | DeepSeek 5.1 | Unique (1/2) | Minor | Confirmed |
| C32 | Empty CLI program maps to setup failure | DeepSeek 5.3 | Unique (1/2) | Minor | Confirmed |
| C33 | Headless local `ask` fails during runtime build | DeepSeek 5.4 | Unique (1/2) | Info | Confirmed, fail-closed and spec-compatible |
| C34 | Wire `Unavailable` loses adapter identity | DeepSeek 7.1 | Unique (1/2) | Info | Confirmed |
| C35 | Mock comments conflict with in-band timeout semantics | DeepSeek 7.2 | Unique (1/2) | Info | Confirmed documentation gap |
| C36 | `Undecided` project trust would be treated as trusted | DeepSeek 7.3 | Unique (1/2) | Info | Confirmed latent guard |
| C37 | Packager SemVer parsing/rendering diverges | DeepSeek 7.4 | Unique (1/2) | Info | Confirmed |
| C38 | Ledger verification predates remediation | DeepSeek 8.1 | Unique (1/2) | Info | Confirmed; excluded by remediation guardrail |
| C39 | Phase-exit evidence is locally absent | DeepSeek 8.2 | Unique (1/2) | Info | Confirmed observation |
| C40 | Package remove is non-transactional; `preserve_trust` is dead | DeepSeek 8.3 residuals | Unique (1/2) | Minor | Confirmed |
| C41 | Residual protocol accounting/bounds/schema/host fixtures | DeepSeek 8.3 residuals | Unique (1/2) | Minor | Confirmed |
| C42 | Permission prompt's impossible cursor fallback allows once | DeepSeek 8.3 residual | Unique (1/2) | Info | Confirmed defensive fail-open |
| C43 | Core attach-failure window can orphan a grandchild | DeepSeek 8.3 residual | Unique (1/2) | Major | Confirmed |
| C44 | Packager `--verify` does not authenticate the archive | DeepSeek 8.3 residual | Unique (1/2) | Minor | Confirmed |
| C45 | Artifact-audit filesystem errors can escape as traceback | DeepSeek 8.3 residual | Unique (1/2) | Minor | Confirmed |
| C46 | Phase 15 docs guard pins deleted paths in present tense | DeepSeek 8.3 residual | Unique (1/2) | Minor | Confirmed |
| C47 | macOS launcher watchdog under abrupt owner death | DeepSeek 8.3 residual | Unique (1/2) | Major candidate | Cannot confirm on Windows; requires native test |

Focused verification completed during planning:

- `cargo test -p opi-protocol`: 59 passed.
- `cargo test -p opi-sandbox --test sdk_contract --test cli_contract --test protocol_conformance`: 76 passed.
- `cargo test -p opi-coding-agent --test execution_package_lifecycle`: 23 passed.
- The feature-enabled handshake-timeout regression passed.

These passes establish the current baseline; they do not clear the missing
negative paths above.

## Decision record

| ID | Finding cluster(s) | Decision | Rationale | Decided by |
|---|---|---|---|---|
| D1 | C06, C31, C41 | Finish the v1 closed/bounded contract and promote key unit invariants to integration fixtures. | The wire already declares these limits; runtime, schema, and fixtures must agree. | auto |
| D2 | C09 | Parse a closed adapter configuration into the standalone policy and reject unsupported values. | Ignoring a trusted configuration field makes the production backend contract false. | auto |
| D3 | C10 | Apply `HostState::transition` on every cancellation-finalization frame. | Frame ordering must not depend on which receive loop is active. | auto |
| D4 | C12, C33 | Add an early default-local/allow production branch; keep explicit interactive `local=ask` as a documented, tested permission-broker exception. | This restores the normative Minimal Runtime without adding a second local permission wrapper. | auto |
| D5 | C17 | Rewind the macOS snapshot before reopening `/dev/fd` and prove add/activate on macOS. | One direct fix restores a first-class target without weakening immutable launch binding. | auto |
| D6 | C18, C19, C21, C22, C23, C34, C35 | Preserve signal and adapter identity, redact unrecognized model input, distinguish store/read/unavailable causes, and share exact remediation wording. | Public diagnostics must be truthful, correlatable, and redacted. | auto |
| D7 | C20, C43 | Preserve every supervision degradation and close the Unix spawn-to-attach escape window. | L0 must fail closed and report cleanup truth on every platform. | auto |
| D8 | C26, C27, C28, C30, C32 | Correct false-positive tests and add real runtime negative/byte-flow coverage. | These are additive tests or a one-condition parser correction with no API choice. | auto |
| D9 | C25 | Add an explicit feature-enabled Phase 16 acceptance CI step and guard its topology. | Default workspace tests compile these suites to empty targets. | auto |
| D10 | C16, C37, C44 | Complete native smoke, unify literal-safe SemVer rendering, and make `--verify` re-extract/authenticate the archive. | Release acceptance must prove the artifact users receive, not caller-owned staging trees. | auto |
| D11 | C29, C45 | Harden phase-exit evidence parsing against ignored tests and filesystem-shape errors. | The auditor should reject bad evidence with structured findings, never accept or traceback. | auto |
| D12 | C36, C40, C42 | Make latent trust/permission fallbacks fail closed and make package removal transactional. | These small changes remove future privilege and lifecycle foot-guns. | auto |
| D13 | C46 | Rephrase the paired Phase 15 text and guard as historical exit evidence. | Historical evidence stays immutable while current docs stop claiming deleted paths exist. | auto |

## Remediation layers

### Layer 1A: `opi-protocol` (substrate)

**Verification**:

    cargo fmt --all
    cargo clippy -p opi-protocol --all-targets -- -D warnings
    cargo test -p opi-protocol --all-targets

#### Fix 1A.1: Finish v1 identity, bounds, and state accounting

- **Audit source**: Codex 2.2; DeepSeek 4.3, 5.1, 8.3 residuals
- **Cluster**: C06, C31, C41
- **Decision**: D1
- **Verification status**: Partially confirmed / Confirmed
- **File(s)**: `crates/opi-protocol/src/execution/v1/identity.rs` ~L120; `bounds.rs` ~L70; `codec.rs` ~L115; `session.rs` ~L80; `schema.rs` ~L45; `mod.rs` ~L70; `tests/execution_v1_contract.rs`; `tests/execution_v1_schema.rs`; protocol fixtures
- **Change**: Reject empty `ProtocolId` values and emit `minLength`; bound `FailedPayload.message`; check request identity before cumulative accounting; correct configuration amplification arithmetic/documentation; remove the internal `SchemaRoot` title; add host-direction, cancel, duplicate, and exact/over-boundary integration fixtures.
- **Test plan**: Empty protocol construction/deserialization/schema tests; failure-message limit and limit+1 tests; cross-request accounting invariant; all five bounds plus duplicate/cancel/unknown-field integration cases.

### Layer 1B: `opi-tui` (substrate)

**Verification**:

    cargo fmt --all
    cargo clippy -p opi-tui --all-targets -- -D warnings
    cargo test -p opi-tui --all-targets

#### Fix 1B.1: Fail closed on an invalid permission cursor

- **Audit source**: DeepSeek 8.3 residual
- **Cluster**: C42
- **Decision**: D12
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-tui/src/permission_prompt.rs` ~L136
- **Change**: Replace the unreachable `AllowOnce` fallback with `Deny` (or an explicit checked error at the caller boundary).
- **Test plan**: Unit-test an injected invalid cursor and assert it cannot authorize an invocation.

### Layer 2: `opi-sandbox` (depends on `opi-protocol`)

**Verification**:

    cargo fmt --all
    cargo clippy -p opi-sandbox --all-targets -- -D warnings
    cargo test -p opi-sandbox --all-targets

#### Fix 2.1: Honor the initialized adapter configuration

- **Audit source**: Codex 2.5
- **Cluster**: C09
- **Decision**: D2
- **Verification status**: Partially confirmed
- **File(s)**: `crates/opi-sandbox/src/backend.rs` ~L125 and ~L380; `helper.rs`; `tests/protocol_conformance.rs`; `tests/backend_protocol_smoke.rs`
- **Change**: Parse the bounded configuration as a closed profile/network object, reject invalid or unknown values before target start, and pass the resulting `SandboxPolicy` into the shared runner instead of always using `default()`.
- **Test plan**: Real backend tests for network deny and allow, invalid/unknown configuration, deadline expiry during setup, and no target start on rejection.

#### Fix 2.2: Close CLI and backend acceptance gaps

- **Audit source**: DeepSeek 4.4, 4.5, 4.7, 5.3
- **Cluster**: C27, C28, C30, C32
- **Decision**: D8
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-sandbox/src/cli.rs` ~L185; `src/backend.rs` ~L270; `tests/sdk_contract.rs` ~L680; `tests/cli_contract.rs` ~L280 and ~L710; `tests/protocol_conformance.rs`
- **Change**: Reject an empty program as usage error; runtime-assert `None/Unrestricted` for the no-restriction runner; pipe actual stdin bytes through the human CLI; drive `ExecutionFailed`; and classify cleanup failures with the intended cleanup phase.
- **Test plan**: Parser and real-binary empty-program exit 2; Linux byte-echo stdin test; exact effective-contract assertion; injected release/stream-end/cleanup failures with exact code and phase.

### Layer 3: `opi-coding-agent` (depends on `opi-protocol` and `opi-tui`)

**Verification**:

    cargo fmt --all
    cargo clippy -p opi-coding-agent --all-targets -- -D warnings
    cargo test -p opi-coding-agent --all-targets

#### Fix 3.1: Restore the production Minimal Runtime

- **Audit source**: Codex 3.1; DeepSeek 5.2, 5.4, 5.5
- **Cluster**: C12, C33
- **Decision**: D4
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-coding-agent/src/harness.rs` ~L150 and ~L975; `src/execution/runtime.rs` ~L185; `tests/execution_minimal_runtime.rs`; `tests/interactive_permission.rs`; text/NDJSON/RPC startup tests
- **Change**: Detect default fixed-local with effective `allow` before constructing or reading the activation store, permission manager/broker, router, or protocol state. Keep explicit interactive `local=ask` routed through the broker, but scope the Branch-1 docs to the default allow path and document headless build-time refusal.
- **Test plan**: Exercise the real harness constructor with unreadable/panic-on-open activation state and construction counters; prove no broker/router/protocol task for default allow; prove explicit interactive ask still supports once/session/deny; assert headless text/NDJSON/RPC return `permission_required` without prompting.

#### Fix 3.2: Correct macOS immutable launch hashing

- **Audit source**: DeepSeek 2.1
- **Cluster**: C17
- **Decision**: D5
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-coding-agent/src/execution/contribution.rs` ~L484; contribution/package lifecycle tests
- **Change**: Seek the copied snapshot to offset zero before reopening `/dev/fd`; retain Linux sealing and the bound descriptor launch path.
- **Test plan**: On macOS, add and activate a package with non-empty executable bytes and assert the declared digest, validated bound bytes, and pre-spawn revalidation all match.

#### Fix 3.3: Enforce state ordering during cancellation

- **Audit source**: Codex 2.6
- **Cluster**: C10
- **Decision**: D3
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-coding-agent/src/execution/protocol_host.rs` ~L880; `tests/execution_protocol_host.rs`; mock backend fixture
- **Change**: Carry the current `HostState` into cancellation finalization and pass every received terminal through the same transition function used by the normal loop.
- **Test plan**: Reject `completed`/`failed` before ready, accepted, and started under cancellation; preserve legal post-start cancellation and cleanup results.

#### Fix 3.4: Preserve truthful, redacted execution diagnostics

- **Audit source**: DeepSeek 2.2, 3.1, 3.3, 3.4, 3.5, 7.1, 7.2
- **Cluster**: C18, C19, C21, C22, C23, C34, C35
- **Decision**: D6
- **Verification status**: Confirmed / Partially confirmed
- **File(s)**: `crates/opi-coding-agent/src/tool/operations.rs` ~L990 and ~L1045; `src/tool/bash.rs` ~L220 and ~L455; `src/execution/router.rs` ~L140; `src/execution/failure.rs` ~L65 and ~L190; `src/execution/protocol_host.rs` ~L800; `src/doctor.rs` ~L510; `src/diagnostic_bridge.rs` ~L270; related diagnostic/migration/product tests
- **Change**: Carry Unix signal number into public operation context and use a signal-specific message; replace unknown model backend text with a safe placeholder; surface activation-store read failure without inventing untrusted records; share the full legacy-migration remediation string; preserve selected adapter identity on wire unavailability; distinguish not-installed from store failure; align mock comments with in-band timeout/cancel semantics.
- **Test plan**: Known-signal local and routed tool results; hostile path/token backend canary across Display/remediation/public diagnostics; corrupt and permission-denied activation store; action/details migration needle parity; wire-unavailable identity and not-installed remediation tests.

#### Fix 3.5: Close supervision degradation gaps

- **Audit source**: DeepSeek 3.2 and 8.3 attach-window residual
- **Cluster**: C20, C43
- **Decision**: D7
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-coding-agent/src/tool/supervision.rs` ~L145 and ~L166; `src/tool/process_tree.rs`; supervision/L0 tests
- **Change**: Retain the Windows `resume_child` error in the degradation vector before cleanup. On Unix, prevent target/descendant execution until tree ownership is established, or terminate the verified process group on attach failure rather than killing only the direct child.
- **Test plan**: Inject resume failure and assert `CODE_PROCESS_TREE_DEGRADED`; force a Unix child to fork during the attach window and prove no descendant survives or holds output pipes.

#### Fix 3.6: Repair the Windows target-mismatch acceptance test

- **Audit source**: DeepSeek 4.2
- **Cluster**: C26
- **Decision**: D8
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-coding-agent/tests/windows_execution_posture.rs` ~L160 and ~L280
- **Change**: Make the synthetic Opi version satisfy the package range, then assert the internal validation detail names target mismatch before checking the public code and no-spawn sentinel.
- **Test plan**: Run the focused test on Windows and retain an adjacent negative version-range test so the two gates cannot mask each other.

#### Fix 3.7: Make package removal and project trust fail closed

- **Audit source**: DeepSeek 7.3 and 8.3 package residuals
- **Cluster**: C36, C40
- **Decision**: D12
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-coding-agent/src/package_cli.rs` ~L335 and ~L610; `src/package_activation.rs` ~L384 and ~L657; `src/main.rs` ~L345 and ~L468; lifecycle and project-trust tests
- **Change**: Snapshot and roll back declaration/lock/trust state around remove; delete the unused `preserve_trust` install parameter and keep preservation solely in the outer transaction; include project config only for explicit `Trusted`, never `Undecided`.
- **Test plan**: Inject trust-store removal failure and assert all package files are unchanged; package install remains untrusted by default; an `Undecided` decision skips project configuration.

### Layer 4: packaging, smoke, artifact audit, and CI

**Verification**:

    cargo fmt --all
    cargo clippy -p opi-coding-agent --all-targets -- -D warnings
    cargo test -p opi-coding-agent --test opi_sandbox_packaging
    cargo test -p opi-coding-agent --test artifact_audit_script
    cargo test -p opi-coding-agent --test opi_sandbox_release_topology

#### Fix 4.1: Run feature-gated Phase 16 acceptance in CI

- **Audit source**: DeepSeek 4.1
- **Cluster**: C25
- **Decision**: D9
- **Verification status**: Confirmed
- **File(s)**: `.github/workflows/ci.yml` ~L45; `crates/opi-coding-agent/Cargo.toml`; `tests/execution_product.rs`; `tests/execution_protocol_host.rs`; `tests/execution_runtime.rs`; CI topology tests
- **Change**: Build `execution_backend_mock` with `--no-run`, then run the product, protocol-host, and runtime targets with `execution-backend-test-fixture`; guard the workflow so the feature cannot silently disappear.
- **Test plan**: Run the exact CI command locally; topology test must find the feature and all three target names.

#### Fix 4.2: Complete and authenticate native archive acceptance

- **Audit source**: Codex 4.2; DeepSeek 7.4 and 8.3 archive-verification residual
- **Cluster**: C16, C37, C44
- **Decision**: D10
- **Verification status**: Partially confirmed / Confirmed
- **File(s)**: `scripts/opi-sandbox-smoke.sh`; `scripts/package-opi-sandbox.sh` ~L80 and ~L160; `scripts/package-opi-sandbox.ps1` ~L75 and ~L135; packaging/smoke tests; release workflow
- **Change**: Add direct setup-failure, empty-working-directory isolation, native filesystem allow/deny, and network deny/allow sentinels against the extracted binary; use one strict SemVer parser and literal-safe manifest rendering in both packagers; make `--verify` independently extract the expected archive into an empty temporary directory and validate exact members and hashes.
- **Test plan**: Linux/macOS extracted-archive runs for every named sentinel; prerelease/build/invalid-metacharacter parser parity; archive tamper with unchanged staging trees must fail verification.

#### Fix 4.3: Harden phase-exit evidence parsing

- **Audit source**: DeepSeek 4.6 and 8.3 artifact-audit residuals
- **Cluster**: C29, C45
- **Decision**: D11
- **Verification status**: Confirmed
- **File(s)**: `scripts/opi-artifact-audit.py` ~L60, ~L975, ~L1035; `crates/opi-coding-agent/tests/artifact_audit_script.rs`
- **Change**: Apply the existing ignored-test rejection to phase-exit gate bundles; convert expected-file `OSError`s and wrong file kinds into structured issues; update stale phase-exit layout comments.
- **Test plan**: Reject `3 passed; 0 failed; 2 ignored`; substitute a directory for each expected scalar file and assert a structured issue with no traceback.

### Layer 5: paired historical documentation (final layer)

**Verification**:

    cargo fmt --all
    cargo clippy -p opi-coding-agent --all-targets -- -D warnings
    cargo test -p opi-coding-agent --test phase15_safety_sandbox_docs
    cargo test -p opi-coding-agent --test phase16_extension_docs

#### Fix 5.1: Make Phase 15 deleted-path claims explicitly historical

- **Audit source**: DeepSeek 8.3 residual
- **Cluster**: C46
- **Decision**: D13
- **Verification status**: Confirmed
- **File(s)**: `docs/opi-spec.md` ~L1985 and ~L2048; `docs/opi-spec.zh.md` counterparts; `crates/opi-coding-agent/tests/phase15_safety_sandbox_docs.rs` ~L240
- **Change**: Rephrase the deleted `sandbox.rs`/`sandbox/windows.rs` unsafe assertions as Phase-15-exit history in English and Chinese; keep current assertions only for files that still exist; update the guard without rewriting archived snapshots.
- **Test plan**: Paired docs guard must require historical wording, current Phase 16 migration wording, and no present-tense claim that deleted paths exist.

## Final verification

    cargo fmt --check --all
    cargo clippy --workspace --all-targets -- -D warnings
    cargo test --workspace --all-targets
    cargo test -p opi-coding-agent --features execution-backend-test-fixture --test execution_backend_mock --no-run
    cargo test -p opi-coding-agent --features execution-backend-test-fixture --test execution_product --test execution_protocol_host --test execution_runtime
    cargo test --workspace --doc
    RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps

Native verification after workspace gates:

1. Run the corrected macOS package add/activate digest regression.
2. Run Linux/macOS native restriction suites and the complete extracted archive smoke.
3. Run the Windows supervision and unsupported-posture suites.
4. Rebuild authenticated four-target `opi-sandbox` archives and run the strengthened artifact audit.

## Scope exclusions

| Finding | Status | Reason |
|---|---|---|
| C01-C05, C08, C11, C13-C15 | Refuted | Current HEAD contains the previous remediation and focused regression evidence. |
| C07 simultaneous stdin-close race | Info/No action | The real host closes stdin only after terminal completion. Changing biased precedence is a protocol decision with no demonstrated production failure; align evidence wording only under Fix 3.4. |
| C24 crate-root `forbid(unsafe_code)` | Info/No action | Audited FFI remains isolated in `process_tree`; a crate-root forbid requires unrelated module/crate restructuring. Keep leaf-module guards and boundary tests. |
| C38 stale archived ledger | Deferred to guarded ledger reconciliation | `opi-remediate` must not modify `.opi-impl-state.json` or the archived snapshot. Record post-remediation verification outside the canonical ledger. |
| C39 absent local phase-exit artifacts | Evidence refresh | Do not fabricate or rewrite historical evidence. Recreate authenticated native evidence through approved CI/native hosts if durable re-audit is required. |
| C47 macOS abrupt-owner watchdog | Cannot confirm | Requires a separate native Seatbelt launcher diagnostic; promote it to a code fix only if reproduced. |
| Trailing-CR line bound | Info/No action | The off-by-one rejects early in the safe direction. |
| Standalone `TreeGuard::attach` non-leader foot-gun | Info/No action | The shipped runner establishes its own process group; no production path passes an arbitrary non-leader PID. |
| macOS profile lossy/special-path residual | Partially confirmed / manual | Current behavior fails toward denial. Exercise non-UTF-8/newline/parenthesis paths in native tests before changing profile serialization. |
| Human CLI 1 MiB buffering/truncation | Info/No action | This is the documented buffered SDK model and not a false success state. |
| PowerShell unsupported marker mismatch | Duplicate/No action | The script also emits the generic marker recognized by the auditor; remove the redundant marker only when touching that script for Fix 4.2. |

No implementation, ledger, commit, push, or release action is authorized by
this plan. Execution begins only after explicit user confirmation.
