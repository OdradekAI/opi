# Phase 16 Pluggable Extensions and Command Execution — Independent Code Audit

**Auditor**: glm5.2 (independent; no prior audit reports consulted)
**Date**: 2026-08-10
**Scope**: Phase 16 registered requirements (design spec + 21 task DoDs/SCs), Tasks 16.1–16.16.3
**Implementation target**: `21dfcd8` (current committed implementation; `git rev-parse HEAD`)
**Phase exit commit**: `f8aff02` (last docs/repo-gate task, 16.16.3) — provenance only
**History use**: provenance and discovery only; no diff coverage boundary
**Method**: Full first-hand read of the core execution layer (runtime, protocol host, L0
supervision, harness wiring, router, failure, permission, contribution validation) plus
builds and per-target test runs at HEAD; a 7-reviewer parallel workflow (opi-protocol
internals, opi-sandbox SDK/platform, cross-crate redaction sweep, tool/package, CI/docs,
security) with per-finding adversarial verification (19 agents, 0 errors). All evidence
derived from committed objects at `21dfcd8`; the only dirty path
(`docs/snapshots/phase16/remediation-plan.md`, a deleted plan doc) is not implementation
evidence.

**Contamination disclosure**: auto-loaded session memory summarized a prior 2026-08-09
re-audit of this phase (a "macOS compile Blocker" re: `tempfile`). That memory was treated
as non-authoritative and re-derived from code; it proved **stale/incorrect at this HEAD**
(see Independence note in §1). Audit context was otherwise uncontaminated: no
`audit.*.md` or evaluator transcript was read before verdict.

---

## 1. Executive Summary

**Verdict: PASS-WITH-FINDINGS**

| Severity | Count |
|----------|-------|
| Blocker  | 0     |
| Major    | 1     |
| Minor    | 0     |
| Info     | 9     |

The Phase 16 implementation is exceptionally clean. Every correctness- and
security-critical path I traced first-hand — the Minimal Runtime, the no-fallback
assembly, the one-shot protocol host state machine, policy-neutral L0 supervision,
contribution hard-gating with validate-then-bind-to-open-fd, fixed/rules/model routing,
the deny/ask/allow permission model with memory-only grants, the 14-code failure
envelope, and the legacy-`[sandbox]`/link-cleanliness migration — is spec-faithful,
well-tested, and carries no code-level defect. A 7-reviewer fan-out with adversarial
verification surfaced **zero** Blocker/Major/Minor code findings; every code-level item
is Info (doc imprecision, test-rigor opportunities, one latent schema/codec range mismatch).

The single Major is **not a code defect**: the spec's *Repository gates* requirement is
unverified at the audit endpoint. `21dfcd8` is 23 commits ahead of `origin/main` and
unpushed, so CI has never run on it; the last pushed CI (`53bc40c`) is red on the
workspace `test` job (all three OSes), `clippy` (ubuntu), and the Linux `Target check`
jobs. The root cause is identified — `error[E0063]: missing field 'backend' in initializer
of 'operations::BashRequest'` (a Phase 16 model-routing field not propagated to all test
sites at that commit) — and HEAD's 5 intervening `fix(execution): remediate` commits plus
local evidence (binary + six Phase-16 test targets + opi-protocol compile and pass at
HEAD) indicate it is very likely already fixed. The action is to push HEAD and confirm
all-OS `test`/`clippy` + six-target CI green before merge.

**Independence note (memory refuted)**: the stale memory's "A1 Blocker" claimed
`tempfile` was a *dev*-dependency used in `contribution.rs:502`, making opi-coding-agent
fail to compile on macOS. Verified false at this HEAD: `tempfile` is a regular
`[dependencies]` entry (`crates/opi-coding-agent/Cargo.toml:62`, under `[dependencies]`
at `:26`, before `[dev-dependencies]` at `:77`), and the `contribution.rs:502` usage is
production `#[cfg(not(target_os = "linux"))]` code — the macOS/non-Linux fallback where
Linux uses a sealed `memfd_create`. The `opi` binary builds on this host (`OPI_EXIT=0`).
Independence was worth the verification.

### Per-task summary

| Task | Title | Verdict |
|------|-------|---------|
| 16.1 | Pin the Phase 16 documentation contract | PASS |
| 16.2 | Pin L0 supervision and define the policy-neutral seam | PASS |
| 16.3 | Add opi-protocol::execution::v1 | PASS |
| 16.4 | Parse and hard-gate executable contributions | PASS |
| 16.5 | Add Package Trust and enable/disable lifecycle | PASS |
| 16.6 | Add execution configuration, failures, routing, and permission policy | PASS |
| 16.7 | Implement the one-shot execution protocol host | PASS |
| 16.8 | Build the deep Execution Runtime assembly | PASS |
| 16.9 | Wire Execution Runtime, dynamic bash schema, and public surfaces | PASS |
| 16.10 | Add the interactive permission broker and TUI prompt | PASS |
| 16.11.1 | Build the standalone opi-sandbox SDK and runner | PASS |
| 16.11.2 | Build the human opi-sandbox CLI and direct smoke | PASS |
| 16.12 | Add the atomic helper gate and protocol backend | PASS |
| 16.13 | Port the Linux native restriction contract | PASS (CI-gated native acceptance) |
| 16.14.1 | Port the macOS native restriction contract | PASS (CI-gated native acceptance) |
| 16.14.2 | Pin the Windows unsupported execution posture | PASS |
| 16.15.1 | Build host-neutral opi-sandbox packaging | PASS |
| 16.15.2 | Wire native package CI, release, and artifact audit | PASS |
| 16.16.1 | Remove core native sandbox and enforce migration boundaries | PASS |
| 16.16.2 | Prove install-to-execute and cross-surface diagnostics | PASS |
| 16.16.3 | Synchronize documentation and close Phase 16 repository gates | PASS-WITH-FINDINGS (repo-gate unverified — see M1) |

---

## 2. Residuals / Integration Findings

### 2.1 Major: Repository-gates requirement unverified at the audit endpoint (HEAD unpushed; origin CI red)

**File:** `.github/workflows/ci.yml` (workspace `test`/`clippy`/`target_check` jobs); audit endpoint `21dfcd8`
**Cause:** The Phase 16 design spec ("Testing and Acceptance > Repository gates") registers
`cargo clippy --workspace --all-targets -- -D warnings`, the workspace test suite, the
six-target compile gate, and the standalone smoke as acceptance requirements, and the
phase-exit ledger records these as met. At the audit endpoint this is unverified:
`git rev-list --count origin/main..HEAD` = **23** — HEAD has never been pushed, so CI has
not run on it. The last pushed CI run (`53bc40c`, 2026-08-04, run 30872117136) is red on
exactly the gate jobs that implement those requirements:

| Job | Status |
|---|---|
| `test (ubuntu/macos/windows-latest)` | failure (all three OSes) |
| `clippy (ubuntu-latest)` | failure |
| `Target check (x86_64-unknown-linux-gnu)` | failure |
| `Target check (aarch64-unknown-linux-gnu)` | failure |
| `doc`, `fmt`, `doctest`, `clippy (macos/windows)`, `opi-sandbox package (ubuntu/macos)`, `Target check (x86_64-apple-darwin/aarch64-apple-darwin/x86_64-pc-windows-msvc/aarch64-pc-windows-msvc)` | success |

The ubuntu `test` failure log shows `error[E0063]: missing field 'backend' in initializer
of 'operations::BashRequest'` — a Phase 16 model-routing field (`BashRequest.backend`,
added by 16.9) not propagated to all struct-literal construction sites at that commit.

**Impact:** The registered Repository-gates requirement cannot be confirmed satisfied at
`21dfcd8`. This is a merge/phase-readiness blocker: the integration branch CI is red on
the core workspace `test` job across all OSes, and the audit endpoint has no green CI run.

**Mitigating evidence (why this is "very likely already fixed," not "known broken"):** HEAD
contains 5 `fix(execution): remediate phase 16 audit findings` commits among the 23 ahead
of origin, plus `5227257` (16.16.1) and `0bf07e7` (16.16.2). At HEAD I verified directly:
`cargo fmt --check --all` clean; `opi`/`opi-sandbox`/`opi-protocol` binaries build; and six
Phase-16 test targets pass (`sandbox_l0` 9/9, `execution_contribution_manifest` 30/30,
`execution_config` 30/30, `execution_routing` 20/20, `execution_permission` 6/6, opi-protocol
all green). The `E0063` field-propagation class of error is precisely what the remediation
commits target, and the binary + sampled tests compiling indicates it is resolved at HEAD.

**Fix:** Push `21dfcd8` (or the merge commit) and confirm the full CI matrix is green —
specifically `test` (all OSes), `clippy (ubuntu)`, and the two Linux `Target check` jobs —
before merge. (Full local workspace `test`/`clippy` was not run for this audit: the Windows
host cannot execute `cfg(unix)` tests and the workspace all-targets run is the
disk-heavy path this host has filled before; the authoritative confirmation is CI.)

```yaml
id: M1-repo-gates-unverified-at-head
source_kind: audit
source_path: docs/snapshots/phase16/audit.glm5.2.md
source_model: glm5.2
independence: fresh-context-same-family
axis: residuals
severity: Major
title: Repository-gates requirement unverified at audit endpoint (HEAD unpushed; origin CI red)
claim: The Phase 16 Repository-gates acceptance requirement is not verifiably met at audit_head 21dfcd8 because HEAD is 23 commits ahead of origin/main and unpushed (no CI run), and the last pushed CI (53bc40c) fails the workspace test job on all three OSes plus clippy(ubuntu) and both Linux Target checks, root-caused to an E0063 missing-field-on-BashRequest compile breakage.
evidence:
  - location: "git rev-list --count origin/main..HEAD"
    detail: "23"
  - location: "gh run view 30872117136 (53bc40c) jobs"
    detail: "test(ubuntu/macos/windows)=failure; clippy(ubuntu)=failure; Target check(x86_64-linux/aarch64-linux)=failure; doc/fmt/doctest/opi-sandbox-package/other target checks=success"
  - location: ".github/workflows/ci.yml run 30872117136 log-failed (test ubuntu)"
    detail: "error[E0063]: missing field `backend` in initializer of `operations::BashRequest`"
criterion_source: "design spec §Testing and Acceptance > Repository gates; task 16.16.3 DoD"
reproduction:
  - "gh run list --branch main --limit 3   # last runs are failure at 53bc40c"
  - "gh run view 30872117136 --log-failed  # E0063 on BashRequest.backend"
  - "git rev-list --count origin/main..HEAD   # 23 (HEAD unpushed)"
confidence: high
status: unverified
```

---

## 3. Info Findings

### 3.1 Info: CompletedPayload.exit schema range (u32) is wider than the codec (≤255)

**File:** `crates/opi-protocol/src/execution/v1/frames.rs:259` (`exit: Option<u32>`); `schema.rs:54-100`; `codec.rs:214-218`
**Cause:** `CompletedPayload.exit` is `Option<u32>` and the generated JSON Schema emits
`{"format":"uint32","minimum":0}` with no `maximum`, but `validate_backend` rejects
`exit > u8::MAX` (255) with `ExitCodeOutOfRange`.
**Impact:** A schema-only non-Rust client could emit exit codes 256..=u32::MAX that pass
schema validation but are rejected by the Rust host codec — a latent interop surprise.
**Fix (optional):** inject `"maximum":255` onto the exit property in `schema_with_bounds`
(mirroring how `maxLength` is injected for diagnostics) so the published schema and codec
agree on the portable range. The project already documents "codec authoritative, schema
necessary-but-not-sufficient," so this is hardening, not a fix.
```yaml
id: I1-exit-schema-range-wider-than-codec
source_kind: audit
source_path: docs/snapshots/phase16/audit.glm5.2.md
source_model: glm5.2
independence: fresh-context-same-family
axis: spec
severity: Info
title: CompletedPayload.exit schema (u32, no maximum) is wider than the codec (<=255)
claim: The published v1 JSON Schema permits exit codes 256..=u32::MAX that the codec rejects (ExitCodeOutOfRange), so a schema-only non-Rust client could construct frames the Rust host refuses.
evidence:
  - location: "crates/opi-protocol/src/execution/v1/frames.rs:259"
    detail: "exit: Option<u32>"
  - location: "tests/snapshots/execution_v1_schema__schema_v1.snap:239-247"
    detail: "exit emits {format:uint32, minimum:0} with no maximum"
  - location: "crates/opi-protocol/src/execution/v1/codec.rs:214-218"
    detail: "validate_backend rejects exit > u8::MAX with ExitCodeOutOfRange"
criterion_source: "design spec §opi-protocol (product-neutral protocol; non-Rust fixture client)"
reproduction:
  - "build a Completed frame with exit=300: jsonschema::validate(&schema(), &frame) succeeds; encode_backend(&frame, &Bounds::DEFAULT) returns Err(ExitCodeOutOfRange)"
confidence: high
status: unverified
```

### 3.2 Info: Session substrate does not enforce terminal-exclusivity (documented runtime responsibility)

**File:** `crates/opi-protocol/src/execution/v1/session.rs:133-142`
**Cause:** `check_duplicate` tracks once-per-execution frames per-kind ("completed" and
"failed" are distinct `HashSet` keys), so a `completed`-then-`failed` (or reverse)
sequence passes the substrate; only a second identical terminal kind is rejected.
**Impact:** None for this substrate — `mod.rs:114-126` explicitly states the substrate does
not enforce full state-machine transition ordering and that exactly-one-terminal is a
host/backend runtime responsibility (enforced by the 16.7 host `transition()`).
**Fix:** none required for 16.3. (Optional: a single "terminal observed" flag would be
stronger, but overlaps the documented host responsibility.)
```yaml
id: I2-session-no-terminal-exclusivity
source_kind: audit
source_path: docs/snapshots/phase16/audit.glm5.2.md
source_model: glm5.2
independence: fresh-context-same-family
axis: invariants
severity: Info
title: Session substrate allows completed-then-failed (documented as runtime responsibility)
claim: The opi-protocol Session accepts a completed frame followed by a failed frame (distinct per-kind keys); terminal-exclusivity is intentionally a host runtime responsibility, not a substrate invariant.
evidence:
  - location: "crates/opi-protocol/src/execution/v1/session.rs:133-142"
    detail: "check_duplicate inserts the frame kind; completed/failed are distinct keys"
  - location: "crates/opi-protocol/src/execution/v1/mod.rs:114-126"
    detail: "substrate documents it does not enforce transition ordering"
  - location: "crates/opi-coding-agent/src/execution/protocol_host.rs:718-755"
    detail: "host transition() enforces terminal ordering; second terminal => ProtocolViolation"
criterion_source: "design spec §State machine; task 16.3 DoD (TERMINAL Failed frame — existence satisfied)"
reproduction:
  - "Session::new(DEFAULT).observe_backend(Completed{rid A}).unwrap(); .observe_backend(Failed{rid A}).unwrap() — both Ok"
confidence: high
status: unverified
```

### 3.3 Info: NativeString rustdoc overstates substrate error mapping

**File:** `crates/opi-protocol/src/execution/v1/native.rs:45-47`
**Cause:** The doc comment says the codec "maps both variants to
`FailureCode::ProtocolViolation` at the session layer," but the substrate actually surfaces
a malformed `NativeString` as `SessionError::Codec(CodecError::Json(...))` (via the serde
custom error at `native.rs:141-145`); the `ProtocolViolation` mapping happens in the 16.7
host runtime, not the session layer.
**Impact:** Documentation only; the error is still caught and propagated correctly.
**Fix (optional):** rephrase to state the substrate surfaces these as `CodecError::Json`
and the host runtime maps to `FailureCode::ProtocolViolation`. (`mod.rs:108` uses the same
loose phrasing, so this is consistency, not a one-off.)
```yaml
id: I3-nativestring-rustdoc-overstates-mapping
source_kind: audit
source_path: docs/snapshots/phase16/audit.glm5.2.md
source_model: glm5.2
independence: fresh-context-same-family
axis: standards
severity: Info
title: NativeString rustdoc claims a session-layer FailureCode::ProtocolViolation mapping that does not exist there
claim: native.rs:45-47 documents a FailureCode::ProtocolViolation mapping "at the session layer," but the substrate surfaces NativeStringError as CodecError::Json; the mapping occurs only in the host runtime.
evidence:
  - location: "crates/opi-protocol/src/execution/v1/native.rs:45-47"
    detail: "doc claim"
  - location: "crates/opi-protocol/src/execution/v1/native.rs:141-145"
    detail: "NativeString::deserialize maps NativeStringError -> serde::de::Error::custom"
  - location: "crates/opi-protocol/src/execution/v1/codec.rs:47-49,168-175"
    detail: "wrapped as CodecError::Json; no FailureCode reference in the substrate"
criterion_source: "task 16.3 DoD (public rustdoc documents compatibility rules)"
reproduction: []
confidence: high
status: unverified
```

### 3.4 Info: doctor --json `target` reports the OS family; protocol `ready` reports the full triple

**File:** `crates/opi-sandbox/src/cli.rs:314-345` (doctor); `crates/opi-sandbox/src/backend.rs:254` (ready)
**Cause:** `DoctorReport.target` is set to `std::env::consts::OS` (e.g. `"linux"`), while
the backend `ready` frame sets `target` to `env!("OPI_SANDBOX_BUILD_TARGET")` (the full
triple, e.g. `x86_64-unknown-linux-gnu`). The cli.rs doc-comment (`:314-316`) documents
this as deliberate ("the OS family that determines the restriction model, not the full
target triple").
**Impact:** None — the design spec lists `target` in the doctor stable object without
defining its value, so this is a defensible, documented implementation choice, not a
deviation. Noted for field-name-parity awareness only.
**Fix:** none required.
```yaml
id: I4-doctor-target-os-family-vs-ready-triple
source_kind: audit
source_path: docs/snapshots/phase16/audit.glm5.2.md
source_model: glm5.2
independence: fresh-context-same-family
axis: spec
severity: Info
title: doctor --json target field is OS family while protocol ready target is the full triple
claim: opi-sandbox doctor reports target as the OS family (std::env::consts::OS) while the protocol ready frame reports the full build triple; both are documented and the spec does not constrain the doctor value.
evidence:
  - location: "crates/opi-sandbox/src/cli.rs:340"
    detail: "DoctorReport.target = std::env::consts::OS"
  - location: "crates/opi-sandbox/src/backend.rs:254"
    detail: "ReadyPayload.target = env!(OPI_SANDBOX_BUILD_TARGET) (full triple via build.rs)"
criterion_source: "design spec §Human CLI (doctor stable object includes target; value unconstrained)"
reproduction: []
confidence: high
status: unverified
```

### 3.5 Info: doctor --json well-formedness is asserted by substring/structural checks, not a JSON parse

**File:** `crates/opi-sandbox/src/cli.rs:387-439`, tests at `cli.rs:586-597`
**Cause:** `doctor_json` hand-builds the JSON string with a local escaper (`json_escape`)
to avoid a production `serde_json` dependency (`serde_json` is a dev-dependency only). The
in-crate and standalone tests assert well-formedness via `starts_with`/`ends_with`/`contains`
rather than parsing the output.
**Impact:** None — every interpolated value is a controlled literal (OS-family constant,
fixed profile/mechanism names, static limitation prose), so the JSON is correct by
construction; `json_escape` covers the RFC-8259-mandated escapes.
**Fix (optional):** add a test that parses `doctor_json` output through
`serde_json::from_str` (dev-dep already available) to lock the schema structurally.
```yaml
id: I5-doctor-json-substring-only-validation
source_kind: audit
source_path: docs/snapshots/phase16/audit.glm5.2.md
source_model: glm5.2
independence: fresh-context-same-family
axis: test-quality
severity: Info
title: doctor --json output validated by substring/structural checks, not a JSON parser
claim: opi-sandbox doctor_json tests assert well-formedness via substring/structural prefix checks rather than parsing the output, because serde_json is intentionally only a dev-dependency.
evidence:
  - location: "crates/opi-sandbox/src/cli.rs:387-412,423-439"
    detail: "doctor_json hand-builds JSON via write! + local json_escape"
  - location: "crates/opi-sandbox/src/cli.rs:586-597"
    detail: "doctor_json_is_well_formed uses starts_with/ends_with/contains, comments 'avoids a serde_json dep'"
criterion_source: "task 16.11.2 DoD (doctor returns the stable object)"
reproduction: []
confidence: high
status: unverified
```

### 3.6 Info: Linux pure-model tests are file-level cfg-gated (unlike macOS)

**File:** `crates/opi-sandbox/src/platform/linux.rs:28` (`#![cfg(target_os = "linux")]`)
**Cause:** `linux.rs` carries a file-level inner cfg attribute, so the entire module —
including its pure-model inline tests (compiled BPF actions, fixed danger blocklist,
`io_uring`-free, AF_UNIX-preserved, target-arch acceptance) — compiles out on every
non-Linux host. `macos.rs` gates only its runtime items and keeps its pure seatbelt-profile
model compilable cross-host.
**Impact:** No spec deviation — the design spec scopes Linux verification to "Supported
Linux runs," and these tests DO run on Linux CI (WSL2/GHA). The asymmetry only means the
Linux pure-model invariants are not exercised on a Windows dev host (where they compile out).
**Fix (optional):** mirror the macOS structure — gate only runtime items
(`LinuxRestriction`, posture, probe) and compile the pure seccomp/Landlock plan builders +
their tests cross-host. The verifier notes this is harder than macOS because the Linux pure
model depends on Linux-only crates (`landlock`, `seccompiler`), so the factor-out is
non-trivial; it is a test-portability improvement, not a correctness gap.
```yaml
id: I6-linux-pure-model-file-level-cfg
source_kind: audit
source_path: docs/snapshots/phase16/audit.glm5.2.md
source_model: glm5.2
independence: fresh-context-same-family
axis: test-quality
severity: Info
title: Linux pure-model tests are file-level cfg-gated, so seccomp/Landlock invariants only run on Linux hosts
claim: linux.rs uses #![cfg(target_os="linux")] at file scope, compiling out its pure-model inline tests (BPF/blocklist/io_uring/AF_UNIX) on non-Linux hosts; macOS structures this so its pure model runs cross-host.
evidence:
  - location: "crates/opi-sandbox/src/platform/linux.rs:28"
    detail: "#![cfg(target_os = \"linux\")]"
  - location: "crates/opi-sandbox/src/platform/macos.rs:18-30"
    detail: "macOS gates only runtime items; pure seatbelt-profile model compiles cross-host"
criterion_source: "design spec §Native platform contract (Linux runs verify on supported Linux)"
reproduction: []
confidence: high
status: unverified
```

### 3.7 Info: manifest_hash uses exact raw bytes, not LF-normalized (sound; ledger characterization inaccurate)

**File:** `crates/opi-coding-agent/src/execution/contribution.rs:277,629-633`
**Cause:** `manifest_sha256_bytes()` hashes the exact raw manifest bytes with no
CRLF→LF normalization; the comment at `:629-630` documents this as deliberate ("exact-byte
identity function so a checkout line-ending change is observable lock drift"). This is a
sound choice (exact-byte hashing maximally detects drift, and packages are installed via
`opi package add`, not git checkout, so cross-platform stability holds). However, the
task-16.4 ledger session note characterizes it as "manifest_hash = SHA-256 over
LF-normalized threaded parsed bytes," which is not what the code does.
**Impact:** No operational defect. The design spec is silent on normalization; the
implementation satisfies "exact SHA-256 match" and "detects drift." The only inaccuracy is
the ledger's evidence description.
**Fix:** either (a) correct the task-16.4 session-note wording to "exact raw bytes," or
(b) leave as-is — the code is defensible. No code change warranted.
```yaml
id: I7-manifest-hash-exact-bytes-ledger-mischar
source_kind: audit
source_path: docs/snapshots/phase16/audit.glm5.2.md
source_model: glm5.2
independence: fresh-context-same-family
axis: residuals
severity: Info
title: manifest_hash uses exact raw bytes (sound); the task-16.4 ledger note mischaracterizes it as LF-normalized
claim: contribution.rs hashes exact raw manifest bytes (no LF normalization), a defensible drift-detection choice the spec permits; the task 16.4 session note describes it inaccurately as "LF-normalized threaded parsed bytes."
evidence:
  - location: "crates/opi-coding-agent/src/execution/contribution.rs:631-633"
    detail: "manifest_sha256_bytes -> sha256_hex(bytes), no normalization"
  - location: "crates/opi-coding-agent/src/execution/contribution.rs:629-630"
    detail: "comment: exact-byte identity function so a checkout line-ending change is observable lock drift"
criterion_source: "design spec §Contribution manifest (silent on normalization; requires exact SHA-256 + drift detection)"
reproduction:
  - "test manifest_hash_changes_for_crlf_byte_drift passes (30/30 in execution_contribution_manifest), confirming exact-byte sensitivity"
confidence: high
status: unverified
```

### 3.8 Info: Ledger records deleted Rust doc-guard test binaries as verification commands

**File:** `docs/snapshots/phase16/opi-impl-state.json:68,2722-2733,3101-3113`
**Cause:** Tasks 16.1, 16.16.1, and 16.16.3 `verification.library_gates` still list
`cargo test -p opi-coding-agent --test phase16_extension_docs` and
`--test phase15_safety_sandbox_docs`, but those Rust test binaries were deleted at commit
`26613ac` and their contract migrated into `scripts/opi-doc-check.py`.
**Impact:** The underlying requirement ("documentation guards pass"; "EN+ZH docs change
together") IS met — `python scripts/opi-doc-check.py` returns exit 0 at HEAD and `ci.yml`
runs it on every push. Only the *recorded evidence path* in the snapshot ledger is stale.
**Fix:** update the three tasks' `verification.library_gates`/`behavioral_tests` in the
snapshot ledger to reference `scripts/opi-doc-check.py` instead of the deleted binaries.
(The snapshot is historical; the live ledger is the one that matters for ongoing work —
verify it already reflects the migration.)
```yaml
id: I8-ledger-stale-docguard-paths
source_kind: audit
source_path: docs/snapshots/phase16/audit.glm5.2.md
source_model: glm5.2
independence: fresh-context-same-family
axis: residuals
severity: Info
title: Snapshot ledger records deleted Rust doc-guard test binaries as verification commands
claim: The phase16 snapshot ledger's verification.library_gates for tasks 16.1/16.16.1/16.16.3 cite cargo test --test phase16_extension_docs / phase15_safety_sandbox_docs, which were deleted at 26613ac and migrated to scripts/opi-doc-check.py; the gate itself passes via the Python script.
evidence:
  - location: "docs/snapshots/phase16/opi-impl-state.json:68"
    detail: "task 16.1 library_gates lists --test phase16_extension_docs"
  - location: "docs/snapshots/phase16/opi-impl-state.json:2722-2733"
    detail: "task 16.16.1 lists --test phase15_safety_sandbox_docs in library_gates and behavioral_tests"
  - location: "scripts/opi-doc-check.py"
    detail: "exit 0 at HEAD; ci.yml:21-25 runs it on push"
criterion_source: "task 16.16.3 DoD (documentation guards pass); design spec §Repository gates (EN+ZH docs change together)"
reproduction:
  - "python scripts/opi-doc-check.py   # exit 0"
confidence: high
status: unverified
```

### 3.9 Info: Bash ToolResult `details` carries the model's own command string (pre-existing Phase 11 behavior, reused by Phase 16)

**File:** `crates/opi-coding-agent/src/tool/bash.rs:258-268` (and `:421-432`); source in `crates/opi-agent/src/tool/result.rs:93-118`
**Cause:** `bash_operation_metadata` (in opi-agent) places the raw `command`,
`workspace_root`, and `cwd` into the ToolResult `details` object; `bash.rs` calls it on the
unified exec path that serves both the local and external adapters with no backend-specific
branching. This is established Phase 11 behavior that Phase 16 deliberately reuses (design
spec §Failure and Diagnostics: "Phase 16 uses each existing public surface's
diagnostic/result envelope").
**Impact:** Informational. The `details` field is model-facing operation metadata
(`ToolResultMessage`), not a public `Diagnostic`; the strict diagnostic envelope
(`bash_operation_diagnostic`, `bash.rs:457-491`) correctly excludes command text ("Raw
command text is intentionally excluded because commands can contain secrets"). The model
needs its own command context to interpret the result it issued. No Phase 16 change is
required.
**Fix (future, out of Phase 16 scope):** if persisted/trace surfaces should redact command
text, distinguish model-facing context (keep) from persisted/trace surfaces (redact) — a
cross-phase tightening, not a Phase 16 regression.
```yaml
id: I9-toolresult-details-carries-command
source_kind: audit
source_path: docs/snapshots/phase16/audit.glm5.2.md
source_model: glm5.2
independence: fresh-context-same-family
axis: security
severity: Info
title: Bash ToolResult details carries the model's own command string on local and external paths (pre-existing Phase 11 behavior)
claim: bash_operation_metadata places the raw command/workspace/cwd into ToolResult details on the unified bash exec path (local + external); this is Phase 11 behavior Phase 16 reuses, and the separate Diagnostic envelope correctly excludes command text.
evidence:
  - location: "crates/opi-agent/src/tool/result.rs:93-118"
    detail: "bash_operation_metadata includes \"command\": command in details"
  - location: "crates/opi-coding-agent/src/tool/bash.rs:258-268"
    detail: "details built from bash_operation_metadata on the external-adapter path, same as local"
  - location: "crates/opi-coding-agent/src/tool/bash.rs:457-491"
    detail: "strict Diagnostic path excludes raw command text"
criterion_source: "design spec §Failure and Diagnostics (Phase 16 reuses existing surfaces; diagnostics omit command text — diagnostics do)"
reproduction: []
confidence: high
status: unverified
```

---

## 4. Invariant Verification

| Invariant (design spec) | Code evidence | Test coverage |
|---|---|---|
| Five independent gates; install≠trust≠enable≠select≠permit | `runtime.rs:316-381` (Branch 1 returns `local_ops` directly; Branch 2 builds eligibility+dispatch); `contribution.rs` distinct error per gate; `permission.rs` deny/ask/allow | `execution_runtime.rs`, `execution_contribution_manifest.rs` 30/30, `execution_permission.rs` 6/6 |
| No fallback to `local` (or any adapter) on selected-external failure | `runtime.rs:444-483` (every selection/adapter failure returns `Err`); `harness.rs:2655` (startup failure omits bash, no substitute) | `runtime.rs` no-local-fallback + PanicStore tests; router `rules_selected_backend_failure_does_not_fall_through` |
| Command not disclosed until `ready` validates | `protocol_host.rs:302-391` (execute frame built+sent after ready identity/version/target match) | `execution_protocol_host.rs` SC16-06a suite |
| Default `bash` schema byte-for-byte pre-extension | `harness.rs:124-149` (`bash_input_schema` returns `default_bash_schema()` for fixed/rules/default); `bash.rs` `production_fixed_strategy_omits_backend_field` | `execution_routing.rs` 20/20 |
| Model routing: deny absent, ask visible-with-description, cannot select disabled/unknown | `router.rs:203-232`, `runtime.rs` `Eligibility::from_enabled` + `bash_input_schema` model enum | `router.rs` `model_cannot_select_denied/absent_backend` |
| Project `[execution.permissions]` rejected (user-owned layer only) | `config.rs:776-782,1098-1112` (`reject_project_execution_permissions` before merge) | `execution_config.rs` 30/30 |
| Current-session grant is memory-only (no restart/resume/fork) | `permission.rs:98-137` (no Serialize; never persisted; `reset_grants` on switch) | `interactive_permission.rs`, `execution_permission.rs` |
| Fail-closed after external adapter selected | `protocol_host.rs` every non-terminal path → terminate_and_fail; `TeardownConfirmation.classify` → CleanupUnconfirmed if unconfirmed | `execution_protocol_host.rs`, `execution_failures.rs` |
| Opi binary does not link opi-sandbox | `crates/opi-coding-agent/Cargo.toml` (no opi-sandbox dep) | `phase16_crate_boundaries.rs`, `crate_boundaries.rs` |
| opi-protocol dependency neutrality | `crates/opi-protocol/Cargo.toml:12-22` (serde/serde_json/schemars/thiserror/base64 only) | `cargo tree -p opi-protocol --no-dev-dependencies` clean |
| Validate-then-bind-to-open-fd (TOCTOU-safe executable launch) | `contribution.rs:131-148` (`bound_launch_path` → `/proc/self/fd` sealed-memfd Linux, `/dev/fd` macOS) | `execution_contribution_manifest.rs` |

**Open acceptance scenarios (positive verification).** All six "open" SCs
(SC16-09b-linux/macos, SC16-10, SC16-11, SC16-12a, SC16-12b, SC16-15a) are legitimately
CI/platform-gated, not unverified gaps: for each, the test code (`linux_policy.rs`,
`macos_policy.rs`, `cli_contract.rs`, `windows_execution_posture.rs`), the CI workflow
wiring (`ci.yml:sandbox_package`, `release.yml:sandbox_archive`), the standalone smoke
script (`scripts/opi-sandbox-smoke.{sh,ps1}`), and the artifact auditor
(`scripts/opi-artifact-audit.py`) all exist and are correctly structured at HEAD. They
carry "open" status only because native Linux/macOS runners cannot close them from a single
Windows dev host — exactly the delegation the design spec ("Native CI and release jobs")
prescribes.

---

## 5. Residuals and Recommendations

### Priority recommendations

1. **(M1) Push HEAD and confirm CI green before merge.** `21dfcd8` is 23 commits ahead of
   `origin/main` and unpushed; origin CI is red on the workspace `test` job (all OSes),
   `clippy(ubuntu)`, and both Linux `Target check` jobs, root-caused to an `E0063` missing
   `backend` field on `BashRequest` at `53bc40c`. Local evidence (binary + six Phase-16
   test targets + opi-protocol compile and pass at HEAD) indicates this is already fixed
   by the intervening remediation commits, but it must be confirmed by a green CI run on
   the merged HEAD. This is the sole action item blocking confident merge; the code itself
   is sound.

2. **(I7, I8) Reconcile snapshot-ledger evidence text.** Correct the task-16.4 manifest_hash
   wording ("exact raw bytes," not "LF-normalized") and replace the deleted
   `phase16_extension_docs`/`phase15_safety_sandbox_docs` Rust test paths in
   tasks 16.1/16.16.1/16.16.3 `verification.library_gates` with `scripts/opi-doc-check.py`.
   Both are documentation-of-evidence accuracy; neither affects runtime.

3. **(Optional hardening, all Info, none blocking)** Add `maximum:255` to the v1 exit-code
   schema (I1); parse `doctor_json` through `serde_json` in a test (I5); factor the Linux
   pure-model tests out of the file-level cfg so they run cross-host (I6); tighten the
   NativeString rustdoc phrasing (I3).

### Dimensions covered

Standards (CLAUDE.md/AGENTS.md + Fowler-smell baseline), Spec (every registered SC/DoD),
Correctness, Security/redaction, Test-quality, Invariants, Cross-task Integration,
Residuals — over the complete relevant implementation at `21dfcd8`, including unchanged and
pre-existing paths (e.g. the Phase 11 `ToolResult.details` behavior in I9).

### What was verified clean (no finding)

Minimal Runtime store-untouchedness; L0 supervision (timeout/cancel/dropped-future/clean-exit
tree-kill, bounded pipe drain, wait-failure termination); the protocol host state machine
and redaction (backend diagnostics neutralized to a placeholder; effective-contract
redaction of exact invocation values; raw backend stderr never surfaced); contribution
hard-gating (closed fields, project-local/opi-range/target/path/symlink/hash/handshake
gates, each a distinct error); routing no-fallthrough and model non-authority; the 14-code
failure envelope with redacted remediation; the memory-only session grant; legacy
`[sandbox]`/`--sandbox`/`--sandbox-require` rejection with actionable remediation; opi
binary not linking opi-sandbox; opi-protocol dependency neutrality; CI topology
consolidation (deleted `sandbox-macos*.yml` correctly folded into `ci.yml:sandbox_package`
and `release.yml:sandbox_archive` plus the six-target `target_check`); and the standalone
opi-sandbox SDK/CLI/backend independence and exit-code mapping.
