# Phase 16 Remediation Plan

**Date**: 2026-08-05
**Audit sources**: `audit.deepseek-v4-flash.md`, `audit.gpt5.md`
**Commit range**: `1021842c937653de545cd335450df985f822bd06..f8aff02`
**Verified code**: `eb7bed84c6e1dea3af7ff391ed65f0dad7282a38`
**Design specs**: `docs/opi-spec.md`, `docs/superpowers/specs/2026-07-28-phase16-pluggable-extension-command-execution-design.md`

---

## Audit cross-reference summary

Two independent reports were available. With two auditors, a finding is either
full consensus (2/2) or unique (1/2); there is no separate majority tier. The
candidate severity is the highest reported severity and was retained unless
code verification justified a lower status.

| Cluster | Theme | Auditors | Consensus | Unified severity | Verification |
|---|---|---|---|---|---|
| C01 | Empty project permission table erases user policy | GPT B-01; DeepSeek 2.1 | Full (2/2) | Blocker (Blocker/Major) | Confirmed |
| C02 | Same-source package update inherits trust across changed bytes | GPT B-02 | Unique (1/2) | Blocker | Confirmed |
| C03 | Native restriction grants the system temp directory | GPT B-03; DeepSeek 3.1 | Full (2/2) | Blocker (Blocker/Minor) | Confirmed |
| C04 | L0 tree attachment fails open while reporting supervision | GPT B-04; DeepSeek 2.3 | Full (2/2) | Blocker (Blocker/Major) | Confirmed |
| C05 | Cleanup failures are ignored and reported as confirmed | GPT B-04 | Unique (1/2) | Blocker | Confirmed |
| C06 | Removing all executable contributions does not invalidate trust | GPT M-01 | Unique (1/2) | Major | Confirmed |
| C07 | Validated package bytes are not bound to the spawned executable | GPT M-02; DeepSeek 3.4 | Full (2/2) | Major (Major/Info) | Confirmed |
| C08 | Unbounded external timeout arithmetic can panic | GPT M-03 | Unique (1/2) | Major | Confirmed |
| C09 | Model schema uses stale candidates and can contain `oneOf: []` | GPT M-04 | Unique (1/2) | Major | Confirmed |
| C10 | Protocol receiver does not enforce all declared bounds | GPT M-05; DeepSeek 4.1, 5.7 | Full (2/2) | Major | Confirmed |
| C11 | Custom `Bounds` arithmetic can overflow | DeepSeek 4.7 | Unique (1/2) | Info | Confirmed |
| C12 | `ready` omits identity and host does not match locked version/target | GPT M-06 | Unique (1/2) | Major | Confirmed |
| C13 | Handshake/configured deadline is unused and phases get fresh grace windows | GPT M-07; DeepSeek 2.10, 2.12 | Full (2/2) | Major | Confirmed |
| C14 | Terminal contamination is discarded and terminal diagnostics are lost | GPT M-08 | Unique (1/2) | Major | Confirmed |
| C15 | In-band backend diagnostics are not host-redacted | DeepSeek 3.2 | Unique (1/2) | Minor | Confirmed |
| C16 | Native strings are converted lossily at Opi and SDK boundaries | GPT M-09 | Unique (1/2) | Major | Confirmed |
| C17 | Target starts before the host observes/flushed `started` | GPT M-10; DeepSeek 5.3 | Full (2/2) | Major | Confirmed |
| C18 | Premature backend stdin EOF lets the target continue | GPT M-11 | Unique (1/2) | Major | Confirmed |
| C19 | Backend protocol input uses an unbounded channel | GPT M-12 | Unique (1/2) | Major | Confirmed |
| C20 | Direct CLI interruption can orphan the target | GPT M-13; DeepSeek 2.2 | Full (2/2) | Major | Confirmed |
| C21 | Effective placement/guarantee/policy/limitations are wrong or hidden | GPT M-14; DeepSeek 2.3 | Full (2/2) | Major | Confirmed |
| C22 | `local = "ask"` omits `bash` in the actual Minimal Runtime | GPT M-15; DeepSeek 5.1 | Full (2/2) | Major | Confirmed |
| C23 | Core drain-grace expiry discards an already-captured prefix | GPT M-16; DeepSeek 2.6 | Full (2/2) | Major (Major/Minor) | Confirmed |
| C24 | Packaged adapter declares an Opi-incompatible `0.8` range | GPT M-17; DeepSeek 4.9, 5.5 | Full (2/2) | Major | Confirmed |
| C25 | Extracted standalone smoke omits full direct/backend contracts | GPT M-18 | Unique (1/2) | Major | Confirmed |
| C26 | Artifact audit can pass without an archive or installable manifest | GPT M-19 | Unique (1/2) | Major | Confirmed |
| C27 | Doctor surfaces replace runtime execution codes/remediation | GPT M-20 | Unique (1/2) | Major | Confirmed |
| C28 | Current docs advertise rejected legacy behavior and stale baseline facts | GPT M-21; DeepSeek 5.4 | Full (2/2) | Major | Confirmed |
| C29 | Nested terminal `Diagnostic` is not schema-closed | GPT N-01 | Unique (1/2) | Minor | Confirmed |
| C30 | Rules selection failure reports strategy `fixed` | GPT N-02 | Unique (1/2) | Minor | Confirmed |
| C31 | Required `AwaitingPermission` status snapshots are absent | GPT N-03 | Unique (1/2) | Minor | Confirmed |
| C32 | Passing ledger tasks retain open acceptance scenarios | GPT N-04 | Unique (1/2) | Minor | Confirmed |
| C33 | Failed package add leaves declaration/lock metadata behind | DeepSeek 2.4 | Unique (1/2) | Minor | Confirmed |
| C34 | Diagnostic drift hashing can block on FIFO/device paths | DeepSeek 2.5 | Unique (1/2) | Minor | Confirmed |
| C35 | Activation swallows durable trust-invalidation write failures | DeepSeek 2.7 | Unique (1/2) | Minor | Confirmed |
| C36 | `opi-sandbox` silently truncates output at 1 MiB | DeepSeek 2.8 | Unique (1/2) | Minor | Confirmed |
| C37 | Backend cancel/completion race is nondeterministic | DeepSeek 2.9 | Unique (1/2) | Minor | Confirmed |
| C38 | SDK `cwd` is not required to be inside `workspace` | DeepSeek 2.11 | Unique (1/2) | Minor | Confirmed |
| C39 | Legacy Phase 5 adapter resolver permits symlink escape | DeepSeek 3.3 | Unique (1/2) | Minor | Confirmed, outside Phase 16 scope |
| C40 | Permission broker production paths lack tests | DeepSeek 4.2 | Unique (1/2) | Minor | Refuted |
| C41 | Signal-exit test cannot distinguish signal from exit 143 | DeepSeek 4.3 | Unique (1/2) | Minor | Confirmed |
| C42 | Supplementary source-text guards remain | DeepSeek 4.4 | Unique (1/2) | Minor | Confirmed, no defect by itself |
| C43 | Malformed-JSON fixture is unused | DeepSeek 4.5 | Unique (1/2) | Info | Confirmed |
| C44 | Invalid base64 deserialization is untested | DeepSeek 4.6 | Unique (1/2) | Info | Confirmed |
| C45 | No-spawn lifecycle test never reaches trusted+enabled activation | DeepSeek 4.8 | Unique (1/2) | Info | Confirmed |
| C46 | Wire `Unavailable` cannot produce `adapter_unavailable` | DeepSeek 5.2 | Unique (1/2) | Minor | Confirmed |
| C47 | CLI execution overrides are not revalidated | DeepSeek 5.6 | Unique (1/2) | Info | Confirmed |

Verification notes:

- C40 is refuted by `tests/interactive_permission.rs`, which exercises
  `AllowOnce`, `AllowSession`, session-grant suppression/reset, denial, the
  `permission_denied` code, and the harness/`ToolResult` chokepoint.
- C32 is real, but `opi-remediate` must not modify the canonical implementation
  ledger. `SC16-10`, `SC16-09b-linux`, `SC16-11`, `SC16-09b-macos`,
  `SC16-12a`, `SC16-12b`, and `SC16-15a` remain `open` while Phase 16 exit is
  recorded as met.
- C42 identifies brittle supplementary tests, but the audited behavioral tests
  still exist. Removing the tripwires alone would not remediate a product defect.

## Decision record

| ID | Finding cluster(s) | Decision | Rationale | Decided by |
|---|---|---|---|---|
| D1 | C01, C30, C47 | Treat any project `permissions: Some(_)` as forbidden, preserve the originating router strategy, and revalidate after CLI overrides. | These are closed configuration invariants with one direct fix each. | auto |
| D2 | C02, C06, C33, C35, C45 | Make package metadata, lock material, and activation state one transactional update; retain trust only when the complete old/new locked contribution sets are byte-identical. | Trust must bind exact material, including contribution removal; failed writes must leave the old state intact or fail closed. | auto |
| D3 | C07, C34 | Validate an opened regular executable and bind the same immutable file identity/material to spawn; diagnostics must reject non-regular files before reading. | This closes both blocking special-file reads and the validate/path/spawn replacement window. | auto |
| D4 | C03, C38 | Carry the exact invocation temp root in `RestrictionCtx`, set `TMPDIR`/`TMP`/`TEMP`, grant only workspace+private-temp writes, and canonicalize/validate `cwd` under workspace. | This is the normative restriction boundary and has no compatible alternative. | auto |
| D5 | C04, C05, C17, C20 | Make L0 attachment fail closed, create/assign Windows children before resume, add a real target-release gate and parent-death containment, and derive cleanup truth from every cleanup step. | A command must never run outside the promised lifecycle boundary or report confirmed cleanup after an unobserved failure. | auto |
| D6 | C10, C11, C29, C43, C44 | Enforce every declared protocol bound on encode/decode, use checked bounds arithmetic, close nested schemas, and add malformed/base64/boundary tests. | These changes make the v1 contract match its own documented closed/bounded surface. | auto |
| D7 | C12, C46 | Add implementation identity to `ready`, match identity/version/target to the selected locked contribution, and map pre-start `Unavailable` to `adapter_unavailable`. | The canonical ready contract requires all three identity fields and the stable failure vocabulary already contains the precise code. | auto |
| D8 | C13 | Use one absolute invocation deadline; cap handshake by the configured sub-deadline and derive execute, cancel, cleanup, drain, and reap windows from the remaining budget. | Fresh grace windows and separate clocks violate the explicit single-deadline contract. | auto |
| D9 | C14, C15, C18, C19, C36, C37 | Require clean EOF immediately after terminal, merge bounded terminal diagnostics after host redaction, make premature input EOF cancel+fail, bound the reader channel, make race precedence deterministic, and report truncation. | This produces a closed one-shot stream with bounded memory and truthful output. | auto |
| D10 | C16 | Carry `OsString`/`PathBuf` losslessly through Opi and the public `opi-sandbox` SDK, with Unix byte and Windows wide-unit conversion tests. | `NativeString` exists specifically to preserve native values; the workspace is pre-stable and does not require compatibility shims. | auto |
| D11 | C08, C09, C22 | Bound tool timeouts with checked arithmetic, resolve eligible compatible candidates before schema construction, omit `bash` with `no_eligible_adapter` when none remain, and route Minimal Runtime `local=ask` through the broker. | These are direct runtime/schema corrections and preserve fail-closed behavior. | auto |
| D12 | C21, C27 | Preserve effective contract fields through `BashResult`, TUI/text, NDJSON, and RPC; reuse `ExecutionFailure` codes/remediation on actionable doctor findings. | Public surfaces must agree on the effective contract and stable diagnostic vocabulary. | auto |
| D13 | C23 | Store stream capture state outside the abortable reader so drain expiry returns the captured prefix. | This directly satisfies the module contract without removing the bounded grace. | auto |
| D14 | C24 | Derive package version and Opi compatibility range from the checked-out workspace/release version in both packagers and tests. | A package built from this tree must be installable by this tree; duplicated future-version constants are unsafe. | auto |
| D15 | C25, C26 | Make smoke exercise exact argv/stdin/stdout/stderr/exit and `backend --stdio`; require the actual archive, auditor-owned extraction, exact layout/manifest/lock/target validation, and archive-bound evidence. | Release evidence must prove the distributable artifact, not a caller-prepared directory or marker. | auto |
| D16 | C28 | Remove legacy Phase 15 configuration from current README/help tables, retain it only in explicitly historical sections, and update EN/ZH current-state headers, crate counts, phase status, and Minimal Runtime wording. | The executable rejects the legacy surface and the current workspace contains six crates. | auto |
| D17 | C31, C41 | Add reviewed permission-status snapshots and a signal-specific exit-status test. | Both are additive coverage with a single direct implementation. | auto |

## Remediation layers

### Layer 1A: `opi-protocol` (substrate)

**Verification**:

    cargo fmt --all
    cargo clippy -p opi-protocol --all-targets -- -D warnings
    cargo test -p opi-protocol --all-targets

#### Fix 1A.1: Close and enforce the v1 bounds/schema contract

- **Audit source**: GPT M-05, N-01; DeepSeek 4.1, 4.5, 4.6, 4.7, 5.7
- **Cluster**: C10, C11, C29, C43, C44
- **Decision**: D6
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-protocol/src/execution/v1/bounds.rs` ~L45; `codec.rs` ~L108; `session.rs` ~L125; `frames.rs` ~L197 and ~L306; `tests/execution_v1_contract.rs`; `tests/execution_v1_schema.rs`
- **Change**: Add checked bound arithmetic; enforce decoded per-chunk size and every nested terminal diagnostic on decode and encode; add `deny_unknown_fields` to nested `Diagnostic`.
- **Test plan**: Exact-limit and limit+1 tests for chunk/config/diagnostic/cumulative bounds; wire the malformed JSON fixture; add invalid-base64 and nested-unknown-field fixtures.

#### Fix 1A.2: Extend ready negotiation identity

- **Audit source**: GPT M-06; DeepSeek 5.2
- **Cluster**: C12, C46
- **Decision**: D7
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-protocol/src/execution/v1/frames.rs` ~L197; valid/invalid ready fixtures and schema snapshots
- **Change**: Add the selected implementation/adapter identity to `ReadyPayload` while retaining version and target as mandatory closed fields.
- **Test plan**: Update shared Rust/non-Rust fixtures and schemas; reject missing, empty, or unknown identity fields.

### Layer 1B: `opi-tui` (substrate)

**Verification**:

    cargo fmt --all
    cargo clippy -p opi-tui --all-targets -- -D warnings
    cargo test -p opi-tui --all-targets

#### Fix 1B.1: Snapshot the awaiting-permission status presentation

- **Audit source**: GPT N-03
- **Cluster**: C31
- **Decision**: D17
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-tui/tests/permission_prompt_snapshots.rs` ~L1; `tests/tui_snapshots.rs` ~L82
- **Change**: Add deterministic public-state renders for `AppStatus::AwaitingPermission` at 80x24 and 120x40, alongside the existing prompt snapshots.
- **Test plan**: Review the generated `.snap.new` files explicitly before accepting the snapshots; run the focused snapshot test.

### Layer 2: `opi-sandbox` (depends on `opi-protocol`)

**Verification**:

    cargo fmt --all
    cargo clippy -p opi-sandbox --all-targets -- -D warnings
    cargo test -p opi-sandbox --all-targets

Native Linux and macOS policy tests are additionally required on their owning
platforms; Windows lifecycle changes require the Windows test target.

#### Fix 2.1: Restrict writes to the invocation-owned temp root

- **Audit source**: GPT B-03; DeepSeek 2.11, 3.1
- **Cluster**: C03, C38
- **Decision**: D4
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-sandbox/src/policy.rs` ~L143; `runner.rs` ~L285; `platform/linux.rs` ~L204; `platform/macos.rs` ~L300
- **Change**: Add `temp_root` to `RestrictionCtx`; set temp environment variables to it; grant only that canonical path; reject a canonical `cwd` outside the canonical workspace.
- **Test plan**: Positive workspace/private-temp writes plus negative sibling-system-temp and outside-cwd tests on Linux/macOS; SDK unit tests for canonical containment and temp environment.

#### Fix 2.2: Make process-tree setup and cleanup truthful

- **Audit source**: GPT B-04, M-10, M-13; DeepSeek 2.2, 2.3, 5.3
- **Cluster**: C04, C05, C17, C20
- **Decision**: D5
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-sandbox/src/runner.rs` ~L340 and ~L480; `process_tree.rs` ~L17 and Windows ~L150; `helper.rs` ~L60; `backend.rs` ~L160; `cli.rs` ~L215; `main.rs`
- **Change**: Fail before target release when L0 attach fails; create Windows targets suspended, assign the Job Object, then resume; add a child bootstrap/release gate so `started` is flushed before the real target can act; add parent-death containment for hard backend termination; propagate termination/wait/drain/temp-removal failures as cleanup unconfirmed; wire SIGINT/Ctrl-C into cancellation and await bounded cleanup before returning 130.
- **Test plan**: Fault-injected attach/terminate/wait/temp failures; target sentinel cannot fire before release; hard-kill backend kills target tree; real SIGINT returns 130 and kills descendants; Windows nested-job tests; macOS full-profile rejection remains pre-start.

#### Fix 2.3: Preserve native strings end to end

- **Audit source**: GPT M-09
- **Cluster**: C16
- **Decision**: D10
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-sandbox/src/runner.rs` ~L75; `helper.rs` ~L90 and ~L175
- **Change**: Change SDK program/args/environment fields to native `OsString`/`PathBuf` forms and replace lossy conversion with platform-correct reversible conversion from `NativeString`.
- **Test plan**: Unix invalid-UTF-8 argv/path/env round trips and Windows unpaired-wide-unit round trips through SDK and protocol backend.

#### Fix 2.4: Bound and close the backend state machine

- **Audit source**: GPT M-07, M-11, M-12; DeepSeek 2.8, 2.9, 2.10
- **Cluster**: C13, C18, C19, C36, C37
- **Decision**: D8, D9
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-sandbox/src/backend.rs` ~L100, ~L205, ~L410, ~L490; `runner.rs` ~L40 and ~L565
- **Change**: Enforce `initialize.deadline_ms`; replace the unbounded input channel with a small bounded channel and stop it after terminal; treat premature EOF/read failure as cancellation plus protocol failure; define deterministic cancel-before-completion precedence; preserve incremental output or emit an explicit bounded truncation diagnostic/flag instead of silent loss.
- **Test plan**: Flood/backpressure test, pre-terminal EOF tree-kill test, deadline-without-execute test, deterministic simultaneous cancel/exit test, and >1 MiB output test asserting a visible truncation marker.

#### Fix 2.5: Strengthen CLI signal-exit coverage

- **Audit source**: DeepSeek 4.3
- **Cluster**: C41
- **Decision**: D17
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-sandbox/tests/cli_contract.rs` ~L672
- **Change**: Make the test prove a signaled `ExitStatus` rather than accepting an ordinary exit code 143.
- **Test plan**: Assert the structured SDK outcome is `Signaled { signal: 15 }` and the CLI maps that result to 143.

### Layer 3: `opi-coding-agent` (depends on `opi-protocol` and `opi-tui`)

**Verification**:

    cargo fmt --all
    cargo clippy -p opi-coding-agent --all-targets -- -D warnings
    cargo test -p opi-coding-agent --all-targets

#### Fix 3.1: Close configuration authority and routing diagnostics

- **Audit source**: GPT B-01, N-02; DeepSeek 2.1, 5.6
- **Cluster**: C01, C30, C47
- **Decision**: D1
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-coding-agent/src/config.rs` ~L185, ~L1065, ~L1227; `execution/router.rs` ~L95; `main.rs` ~L351 and ~L471
- **Change**: Reject project permission table presence even when empty; make named selection carry the originating strategy; run execution-config validation after applying CLI overrides.
- **Test plan**: User deny plus empty project table at both project merge sites; explicit empty user/CLI tables remain valid where owned; rules-selected missing backend reports `rules`; invalid rules override fails as config before runtime construction.

#### Fix 3.2: Make package install/update and activation atomic

- **Audit source**: GPT B-02, M-01, M-02; DeepSeek 2.4, 2.5, 2.7, 3.4, 4.8
- **Cluster**: C02, C06, C07, C33, C34, C35, C45
- **Decision**: D2, D3
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-coding-agent/src/package_cli.rs` ~L120, ~L220, ~L866; `package_activation.rs` ~L385, ~L490, ~L566, ~L608; `execution/contribution.rs` ~L375; `execution/runtime.rs` ~L493; `execution/protocol_host.rs` ~L155
- **Change**: Stage declaration/lock/trust changes and publish them atomically after collision/activation validation; compare complete old/new lock sets before preserving trust; make zero-contribution updates invalidate trust; surface trust-file invalidation write errors; reject special executable files before diagnostic reads; bind the validated open immutable executable identity/material to spawn.
- **Test plan**: Local and Git byte-change updates reset trust+enablement; byte-identical re-add preserves both; trusted+enabled to zero contributions resets both; collision/write failure leaves declaration/lock/trust/cache unchanged; FIFO/device doctor returns promptly; a replacement race cannot spawn unvalidated bytes; trusted+enabled activation still never starts package code.

#### Fix 3.3: Build only usable runtime/schema candidates

- **Audit source**: GPT M-03, M-04, M-15; DeepSeek 5.1
- **Cluster**: C08, C09, C22
- **Decision**: D11
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-coding-agent/src/tool/bash.rs` ~L110 and ~L175; `execution/runtime.rs` ~L145, ~L194, ~L255; `harness.rs` ~L120 and ~L2315
- **Change**: Put a finite timeout maximum in schema/deserialization and use checked deadline arithmetic; resolve current target/version/hash compatibility before model-schema construction; never emit an empty backend `oneOf`; route the no-extension Minimal Runtime `local=ask` through the installed broker.
- **Test plan**: `u64::MAX` and limit+1 return stable tool failures without panic; stale/mismatched identities do not appear in schema; zero candidates omit `bash` with `no_eligible_adapter`; exact production local-only `ask` supports allow-once/session/deny.

#### Fix 3.4: Enforce negotiation, deadline, terminal, and diagnostic contracts in the host

- **Audit source**: GPT M-06, M-07, M-08, M-09, M-14; DeepSeek 2.12, 3.2, 5.2
- **Cluster**: C12, C13, C14, C15, C16, C21, C46
- **Decision**: D7, D8, D9, D10, D12
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-coding-agent/src/execution/protocol_host.rs` ~L70, ~L175, ~L275, ~L425, ~L490, ~L620; `execution/runtime.rs` ~L545; `tool/bash.rs` ~L260
- **Change**: Pass expected identity/version/target and configured handshake timeout into the host; build native paths losslessly; use the single absolute deadline; reject bytes/frames after terminal and require immediate clean EOF; merge terminal diagnostics; redact all in-band diagnostics at the host boundary; map pre-start unavailable precisely; preserve ready/started effective-contract fields through `BashResult` instead of dropping/filtering them.
- **Test plan**: Mismatched identity/version/target, slow handshake, cleanup consuming the remaining budget, terminal extra frame/raw byte, terminal diagnostic merge, hostile path/secret diagnostic redaction, native path round trip, and cross-surface effective-contract tests.

#### Fix 3.5: Retain captured output across drain expiry

- **Audit source**: GPT M-16; DeepSeek 2.6
- **Cluster**: C23
- **Decision**: D13
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-coding-agent/src/tool/supervision.rs` ~L245
- **Change**: Move bounded capture state outside the abortable drain task and snapshot the prefix after abort/grace expiry.
- **Test plan**: Write a prefix, keep the descendant pipe open past 500 ms, and assert the prefix plus the expected degradation survives.

#### Fix 3.6: Reuse execution diagnostics on doctor surfaces

- **Audit source**: GPT M-20
- **Cluster**: C27
- **Decision**: D12
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-coding-agent/src/doctor.rs` ~L200 and ~L509; `package_cli.rs` ~L390; `execution/failure.rs` ~L115
- **Change**: Emit the applicable stable `ExecutionFailure` code and remediation for actionable lifecycle/drift failures; keep doctor-local codes only for summaries and informational observations.
- **Test plan**: Runtime, `package doctor --json`, and `opi doctor --json` produce correlatable code/remediation for the same drift/untrusted/disabled condition; text output remains redacted.

### Layer 4: packaging, smoke, and artifact evidence

**Verification**:

    cargo fmt --all
    cargo clippy -p opi-coding-agent --all-targets -- -D warnings
    cargo test -p opi-coding-agent --test opi_sandbox_packaging
    cargo test -p opi-coding-agent --test artifact_audit_script

The extracted direct/backend smoke must additionally run against fresh Linux and
macOS archives; Windows retains the unsupported/no-artifact posture test.

#### Fix 4.1: Derive package compatibility from the workspace/release version

- **Audit source**: GPT M-17; DeepSeek 4.9, 5.5
- **Cluster**: C24
- **Decision**: D14
- **Verification status**: Confirmed
- **File(s)**: `packaging/opi-sandbox/package.toml.template` ~L16; `scripts/package-opi-sandbox.sh` ~L165; `scripts/package-opi-sandbox.ps1` ~L165; packaging/product fixtures
- **Change**: Remove hard-coded `0.8.0`/`>=0.8,<0.9`; derive the package version and compatible Opi semver range once and use it in manifest, audit lock, and tests.
- **Test plan**: Validate the generated package with `host_opi_version()` from the same checkout; retain negative adjacent-range tests.

#### Fix 4.2: Complete extracted standalone smoke

- **Audit source**: GPT M-18
- **Cluster**: C25
- **Decision**: D15
- **Verification status**: Confirmed
- **File(s)**: `scripts/opi-sandbox-smoke.sh`; `scripts/opi-sandbox-smoke.ps1`; protocol fixture host/evidence markers
- **Change**: Run an explicit target that proves argv, stdin, binary stdout, binary stderr, normal/nonzero/signal/timeout exits, and run `backend --stdio` through a product-neutral client against the extracted binary.
- **Test plan**: Separate direct and backend evidence markers; no workspace binary/cargo/`opi` fallback; no durable state or Opi sentinel access.

#### Fix 4.3: Make artifact audit own and authenticate the evidence

- **Audit source**: GPT M-19
- **Cluster**: C26
- **Decision**: D15
- **Verification status**: Confirmed
- **File(s)**: `scripts/opi-artifact-audit.py` ~L427, ~L446, ~L579; `crates/opi-coding-agent/tests/artifact_audit_script.rs`; packaging verification tests
- **Change**: Require the target archive; extract it into an auditor-owned empty directory; reject traversal/extra/missing layout; parse the real contribution manifest; compare all lock fields, target, manifest hash, executable hash, and archive digest; bind smoke evidence to that digest; inspect skip/failure evidence before any pass marker; remove the no-archive native phase-exit exception.
- **Test plan**: Negative fixtures for absent/tampered archive, caller-prepared extracted tree, placeholder/invalid manifest, wrong target/layout/lock field, mixed PASS+failure log, and evidence for a different archive.

### Layer 5: current product documentation (final layer)

**Verification**:

    cargo fmt --all
    cargo clippy -p opi-coding-agent --all-targets -- -D warnings
    cargo test -p opi-coding-agent --test phase16_extension_docs
    cargo test -p opi-coding-agent --test phase15_safety_sandbox_docs

#### Fix 5.1: Separate current Phase 16 behavior from historical Phase 15

- **Audit source**: GPT M-21; DeepSeek 5.4
- **Cluster**: C28
- **Decision**: D16
- **Verification status**: Confirmed
- **File(s)**: `README.md` ~L150 and ~L390; `README.zh.md` counterparts; `docs/opi-spec.md` ~L9, ~L29, ~L174, ~L2036; `docs/opi-spec.zh.md` counterparts; Phase 16 doc guards
- **Change**: Remove legacy flags/config from current option and safety guidance; retain Phase 15 text only under an explicit historical heading; update current implementation/next milestone/completed phases and the six-crate workspace; change Minimal Runtime wording from “touches no package-store sentinel” to the precise “no package activation or per-package scan” behavior; synchronize English and Chinese.
- **Test plan**: Guard current sections against legacy instructions/stale four-crate/Phases-1-15 claims while explicitly allowing archived Phase 15 history; assert EN/ZH current-state equivalence.

## Final verification

    cargo fmt --check --all
    cargo clippy --workspace --all-targets -- -D warnings
    cargo test --workspace --all-targets
    cargo test --workspace --doc
    RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps

Native/release verification after workspace gates:

1. Build fresh `opi-sandbox` archives on Linux x86_64/aarch64 and macOS
   x86_64/aarch64.
2. Run the complete extracted direct and `backend --stdio` smoke against each
   archive.
3. Run native Linux/macOS restriction suites and the Windows L0/unsupported
   posture suite.
4. Run the strengthened artifact auditor from the archives and their bound
   evidence, not from pre-extracted directories.

## Scope exclusions

| Finding | Status | Reason |
|---|---|---|
| GPT N-04 / C32 | Deferred to guarded ledger reconciliation | The finding is confirmed, but `opi-remediate` is forbidden from modifying `docs/snapshots/phase16/opi-impl-state.json`. Reconcile the open scenarios through `opi-implement` after remediation evidence exists. |
| DeepSeek 3.3 / C39 | Deferred to Phase 19 | The legacy `opi-extension-jsonl-v1` resolver issue is confirmed but is a pre-existing Phase 5 surface explicitly outside the Phase 16 migration/non-goals. Track it with the broader extension-architecture work. |
| DeepSeek 4.2 / C40 | Refuted | `tests/interactive_permission.rs` already tests the real broker decisions, grant lifetime/reset, `permission_denied`, and the harness result surface. |
| DeepSeek 4.4 / C42 | Info/No action | The source-text assertions are supplementary tripwires backed by behavioral tests. Do not remove them as unrelated cleanup; replace only when a stronger behavioral proof is added for the same property. |

No implementation, ledger, commit, push, or release action is authorized by
this plan. Execution begins only after explicit user confirmation.
