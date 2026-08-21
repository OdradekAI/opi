# Phase 17 Deep Agent Core Semantic Closure -- Independent Code Audit

**Auditor**: glm5.3 (independent audit pass; repo audit/evaluator files not consulted before findings were final)
**Date**: 2026-08-21
**Scope**: Phase 17 registered requirements (design doc + tightened `docs/opi-spec.md` at HEAD) and Tasks 17.1--17.9
**Implementation target**: `a680c5df13a08d5a2abc48b482a69d1c594f288e` (current committed implementation; worktree clean except one externally modified audit report file, excluded from evidence)
**Phase exit commit**: `a4cfa4ddc74b4dfac59b4305d4657599af866480` (provenance only)
**History use**: provenance and discovery only; no diff coverage boundary
**Method**: requirements matrix built from the archived ledger (70 criteria rows, 9 task DoDs, inference notes) and both registered specs read in full at `audit_head`; 13 independent dimension finders (deep file-group reads over ~35k source + ~25k directly relevant test lines; 9 in one workflow, 4 recovered after GLM 429 rate-limit failures); 2 Blocker/Major candidates adversarially verified by dedicated refuters; the 2 Majors arriving after that stage were re-verified line-by-line by the auditor; local verification executed at `audit_head`: 621 tests across 26 phase17-focused test binaries (all green, Windows), `cargo fmt --check --all` PASS, `cargo clippy -p opi-agent -p opi-ai -p opi-coding-agent --lib -- -D warnings` PASS, `python scripts/opi-doc-check.py` PASS; CI history inspected via `gh`.
**Independence disclosure**: the auditor retains cross-session memory of Phase 17 implementation sessions; no `audit.*.md`, `remediation-plan*`, evaluator, or adjudication files were opened before this report was final. Treat as reduced-independence, same-family. A concurrent external modification of `docs/snapshots/phase17/audit.codex.md` appeared in the worktree mid-audit; it was never read and is not evidence.

---

## 1. Executive Summary

**Verdict: PASS-WITH-FINDINGS**

| Severity | Count |
|----------|-------|
| Blocker  | 0     |
| Major    | 4     |
| Minor    | 24    |
| Info     | 28    |

The Phase 17 semantic core is genuinely implemented and behaviorally pinned at
`audit_head`: all 70 registered criteria are substantiated in current code (67
fully met; PLT-001/A15 and the platform legs of RBK-001 are *partial* only
because their three-platform CI evidence attaches to the exit SHA `40f2e6e`,
not to HEAD), the Matt Spec and Standards axes found no unimplemented
requirement and no non-goal implemented, and this audit independently
re-ran the focused acceptance suites at HEAD with zero failures. The four
Majors are: two evidence-currency defects (the 5 unpushed post-exit commits --
including an ~8k-line test rewrite and ~15k-line source rewrite in `a680c5d`
alone -- have no CI and no recorded full-suite validation, so every ledger
"ran green" claim predates the code being audited) and two real behavioral
defects introduced by the post-exit remediation in the model-change write path
(`set_model_validated` persists a route the runtime then rejects, leaving
durable/runtime divergence; `apply_recorded_model` panics via `.expect` when
a recorded route is registry-resolvable but not dispatchable). Neither
behavioral Major corrupts user data or is reachable without an extension
provider contributing a lookup-only route, but both violate the phase's own
typed-failure and write-side contracts and should be fixed before the next
phase.

### Per-task summary

| Task | Title | Verdict |
|------|-------|---------|
| 17.1 | Add collection-owned route and authentication preparation | PASS |
| 17.2 | Cut over Agent to durable atomic NextTurnState | PASS |
| 17.3 | Define evidence identities, health, and storage-neutral lifecycle | PASS |
| 17.4 | Cut over trusted tool registrations and mandatory authorization | PASS |
| 17.5 | Wire the Reference Product to dispatchable provider routes | PASS-WITH-FINDINGS (M3) |
| 17.6 | Expand Agent evidence runtime over stable identities | PASS |
| 17.7 | Cut over Reference Product evidence, finalization, and redaction | PASS |
| 17.8 | Migrate legacy session routes and preserve opaque trace artifacts | PASS-WITH-FINDINGS (M4) |
| 17.9 | Close local cross-mode, failure, rollback, documentation, and CI acceptance | PASS-WITH-FINDINGS (M1, M2, MIN-11) |

Positive corrections to the archived ledger (code stronger than recorded):
the PRV-005 "Static registration" residual was materially narrowed by
unpushed `211aba8` (resolver-reported provenance now survives
`prepare_call`; only genuinely static resolvers label Static), and both
documented cross-mode asymmetries (interactive no capture; RPC no `--trace`
forwarding) were closed -- all product modes forward `--trace` at HEAD.

---

## 2. Standards Findings

Matt Standards axis: repo standards (`CLAUDE.md`/`AGENTS.md`) plus the Fowler
smell baseline as judgement calls. The load-bearing standards are respected
(thiserror-only library errors, no `unsafe`, inward dependencies, fail-closed
validation); the findings below are hygiene and duplication clusters.

### 2.1 MINOR: Phase/task history preserved in production source comments

**File:** `crates/opi-agent/src/hooks.rs`
**Lines:** 21, 52, 63, 124 (+11 more sites across crates)
**Cause:** Phase 17 introduced `Phase 17.x` / `17.N` / `pre-17.5` markers into
doc and inline comments (e.g. `hooks.rs:21` "Phase 17.4 (AUT-006): this is a
non-authoritative observation hook"; `agent.rs:572` "(Phase 17.4 adds the
authorizer binding)"; `evidence.rs:133,226,1792`; `extension.rs:246,676`;
`sdk.rs:118`; `registry.rs:277`; `adapter_extension.rs:497`;
`execution/permission.rs:79`).
**Impact:** Violates the documented rule "Do not preserve Phase, task, PR, or
review history in source comments"; the comments will mislead readers once
the phase numbers lose context.
**Fix:** Rewrite the 12 marked comments to state the current contract only
(keep the invariant text, drop the phase/task references).

```yaml
id: MIN-01
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: standards
severity: Minor
title: Phase/task history preserved in production source comments
claim: At least 12 production source comments across opi-agent, opi-ai, and opi-coding-agent cite Phase 17 task numbers, violating the documented no-history-in-comments rule.
evidence:
  - location: crates/opi-agent/src/hooks.rs:21,52,63,124
    detail: Doc comments beginning "Phase 17.2:" / "Phase 17.4 (AUT-006):"
  - location: crates/opi-agent/src/agent.rs:572
    detail: "// Agent construction seam (Phase 17.4 adds the authorizer binding)"
  - location: crates/opi-agent/src/evidence.rs:133,226,1792
    detail: "The loop (17.6) holds it" / "authorizers receive a *copy* ... (17.4)" / "(17.7)"
criterion_source: CLAUDE.md "Comments and rustdoc describe current contracts ... Do not preserve Phase, task, PR, or review history"
reproduction: []
confidence: high
status: unverified
```

### 2.2 MINOR: CLAUDE.md lost its Claude Code flavor; the four-flavor lockstep contract is now self-false

**File:** `CLAUDE.md`
**Lines:** 1--3, 33, 207--208
**Cause:** Commit `eb5e316` ("centralize agent guidance") rewrote CLAUDE.md to
be byte-identical to `AGENTS.md` (title `# AGENTS.md`, audience "guidance to
Codex", `Co-Authored-By: Codex` flavor), dropping all four intentional
tool-flavor phrases, while both files still mandate "Keep `AGENTS.md` and
`CLAUDE.md` identical except for their four intentional tool-flavor phrases."
**Impact:** The repo's own documentation contract is unsatisfiable as written;
CLAUDE.md addresses the wrong tool.
**Fix:** Either restore the four Claude flavor phrases in CLAUDE.md or revise
the lockstep sentence in both files to match the new single-content policy.

```yaml
id: MIN-11
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: standards
severity: Minor
title: CLAUDE.md/AGENTS.md four-flavor lockstep contract is self-false after eb5e316
claim: CLAUDE.md is byte-identical to AGENTS.md at HEAD, so the "four intentional tool-flavor phrases" contract both files state is violated.
evidence:
  - location: CLAUDE.md:1-3
    detail: Title "# AGENTS.md"; audience line names Codex; cmp/md5 identical to AGENTS.md
  - location: CLAUDE.md:33
    detail: "Keep AGENTS.md and CLAUDE.md identical except for their four intentional tool-flavor phrases."
criterion_source: CLAUDE.md/AGENTS.md lockstep clause
reproduction: []
confidence: high
status: unverified
```

### 2.3 MINOR: Six adapters attach `ResolvedAuth.secret` unconditionally, ignoring the documented AuthScheme attachment contract

**File:** `crates/opi-ai/src/openai_chat.rs`
**Lines:** 1145--1154 (representative; also azure_openai, gemini, vertex, openai_responses, openai_codex_responses, api_mapped)
**Cause:** `AuthScheme` is documented as how a concrete provider attaches the
secret at its HTTP boundary, and Anthropic and Bedrock enforce it fail-closed
(`anthropic.rs:957-972` rejects `AwsSigV4`; `bedrock/mod.rs:213-222` requires
it). The other six adapter paths attach the secret unconditionally without
scheme discrimination.
**Impact:** An embedder preparing a mismatched scheme for those wires gets a
wire-shaped request instead of a typed config error; defence-in-depth
inconsistency between adapters.
**Fix:** Mirror the Anthropic/Bedrock scheme check (or a shared helper) in the
six remaining `stream_prepared` paths.

```yaml
id: MIN-07
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: standards
severity: Minor
title: Six adapters attach the auth secret unconditionally instead of enforcing AuthScheme fail-closed
claim: Only Anthropic and Bedrock validate the prepared AuthScheme before attaching the secret; the other six production adapter paths attach ResolvedAuth.secret unconditionally.
evidence:
  - location: crates/opi-ai/src/auth.rs:59-73
    detail: AuthScheme documented as the HTTP-boundary attachment contract
  - location: crates/opi-ai/src/anthropic.rs:957-972
    detail: Fail-closed scheme mismatch rejection (positive pattern)
  - location: crates/opi-ai/src/openai_chat.rs:1145-1154
    detail: Unconditional secret attachment without scheme check
criterion_source: P17-PRV-006
reproduction: []
confidence: medium
status: unverified
```

### 2.4--2.9 Remaining Standards findings (summary)

Each is a Fowler-baseline judgement call or documented-standard hygiene gap
without behavioral impact; full normalized blocks follow the same contract.

- **MIN-02 Duplicated Code -- sequential vs parallel tool arms** (`crates/opi-agent/src/agent_loop.rs:507-651` vs `652-866`): the result-completion pipeline (event emission, `ToolResultMessage` assembly, outcome emission, context push) is duplicated across the two arms, and `ToolEvidenceContext` is assembled at four sites (`:522-532, 614-622, 678-688, 845-853`). Extract a shared completion helper.
- **MIN-03 Duplicated Code -- `InMemoryEvidenceSink`** (`crates/opi-agent/src/evidence.rs:2364-2386` vs `2509-2522`): inherent `records()/has_failure()/completed_manifest()` duplicate the `EvidenceRecorder` trait impl bodies verbatim; the inherent methods should delegate (or vanish).
- **MIN-04 Duplicated Code -- session-adoption tails triplicated** (`crates/opi-coding-agent/src/harness.rs:2258-2300, 2451-2487, 2562-2594`): `resume_session_id`, `fork_current_session`, `resume_session_branch_tip` repeat the same ~30-line adoption sequence; extract `adopt_session_entries`.
- **MIN-05 Data Clumps -- telescoping constructors** (`crates/opi-coding-agent/src/rpc.rs:131-340`; `runner.rs:112-247`): five/six stacked positional constructors threading `trace_path`/`auth_resolver`/`extra_routes`; the builder surface already exists for `CodingHarness`.
- **MIN-06 Speculative seam -- `register_extension_tools`** (`crates/opi-coding-agent/src/tool_authority.rs:96-128`): exported with zero production callers, and `register_product_tools` accepts `_extension_tools` it unconditionally discards; an extension-permission filter shaped for a future need.
- **INFO-01 Dual read paths on `FinalizedManifest`** (`crates/opi-agent/src/evidence.rs:1850-1858, 2124-2130`): both `facts()` and `Deref<Target = ManifestCandidate>`.
- **INFO-02 `Agent::new` parses the model spec twice** (`crates/opi-agent/src/agent.rs:583-594`): `split_once(':')` then `parse_model_spec` which subsumes it.
- **INFO-03 Comments cite volatile design-spec line numbers** (`crates/opi-coding-agent/src/tool_authority.rs:58-59, 146-152`): "spec lines 419-423" etc.; cite section anchors instead.
- **INFO-04 gemini/azure/vertex omit the in-adapter capability preflight** the other five adapters perform (`gemini.rs:859-898`, `azure_openai.rs:220-260`, `vertex.rs:157-197`); the collection-level `validate_request_for_model` still covers dispatched calls, so direct library calls alone skip it.

```yaml
id: MIN-02
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: standards
severity: Minor
title: Sequential and parallel tool arms duplicate the result-completion pipeline
claim: agent_loop duplicates the tool result completion pipeline across its sequential and parallel arms and assembles ToolEvidenceContext at four sites.
evidence:
  - location: crates/opi-agent/src/agent_loop.rs:592-633 vs 824-864
    detail: Identical emission/ToolResultMessage/outcome/context tail in both arms
  - location: crates/opi-agent/src/agent_loop.rs:522-532,614-622,678-688,845-853
    detail: ToolEvidenceContext assembled from the same fields at four sites
criterion_source: CLAUDE.md working principles (duplication/reuse)
reproduction: []
confidence: high
status: unverified
```

```yaml
id: MIN-03
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: standards
severity: Minor
title: InMemoryEvidenceSink inherent query methods duplicate the trait impl bodies
claim: InMemoryEvidenceSink implements records()/has_failure()/completed_manifest() twice with identical bodies (inherent + EvidenceRecorder trait).
evidence:
  - location: crates/opi-agent/src/evidence.rs:2364-2386 vs 2509-2522
    detail: Two copies of the same three method bodies including the has_failure-implies-None manifest rule
criterion_source: null
reproduction: []
confidence: high
status: unverified
```

```yaml
id: MIN-04
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: standards
severity: Minor
title: Session-adoption tail triplicated across resume/fork/branch-switch
claim: resume_session_id, fork_current_session, and resume_session_branch_tip re-implement the same ~30-line session adoption sequence.
evidence:
  - location: crates/opi-coding-agent/src/harness.rs:2258-2300,2451-2487,2562-2594
    detail: reconstruct_context -> replace_agent_context -> apply_recorded_model/thinking -> diagnostics -> open_existing -> sync sequence repeated with only the session source varying
criterion_source: null
reproduction: []
confidence: medium
status: unverified
```

```yaml
id: MIN-05
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: standards
severity: Minor
title: Telescoping positional constructors on RpcRunner and NonInteractiveRunner
claim: RpcRunner and NonInteractiveRunner accumulated five/six stacked positional constructors threading trace/auth/extra-route parameters, dominated by None/Vec::new() passthroughs.
evidence:
  - location: crates/opi-coding-agent/src/rpc.rs:131,166,200,234,276,322
    detail: Six #[allow(clippy::too_many_arguments)] constructors; new_with_optional_extension_registry takes 16 positional parameters
  - location: crates/opi-coding-agent/src/runner.rs:112,140,176,212,247
    detail: Five mirrored constructors on NonInteractiveRunner
criterion_source: null
reproduction: []
confidence: medium
status: unverified
```

```yaml
id: MIN-06
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: standards
severity: Minor
title: Speculative register_extension_tools seam with discarded parameter
claim: register_extension_tools is exported with zero production callers and register_product_tools unconditionally discards its _extension_tools parameter.
evidence:
  - location: crates/opi-coding-agent/src/tool_authority.rs:123-128
    detail: register_product_tools(builtin_tools, _extension_tools) ignores its second parameter
  - location: crates/opi-coding-agent/src/tool_authority.rs:96-116
    detail: register_extension_tools consumed only by phase17_tool_authority.rs:467
criterion_source: CLAUDE.md "Do not add ... one-use abstractions, speculative configurability"
reproduction: []
confidence: medium
status: unverified
```

```yaml
id: INFO-01
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: standards
severity: Info
title: FinalizedManifest exposes both facts() and Deref read paths
claim: FinalizedManifest provides redundant facts() and Deref<Target=ManifestCandidate> read surfaces.
evidence:
  - location: crates/opi-agent/src/evidence.rs:1850-1858,2124-2130
    detail: facts() accessor plus Deref impl over the same data
criterion_source: null
reproduction: []
confidence: high
status: unverified
```

```yaml
id: INFO-02
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: standards
severity: Info
title: Agent::new parses the canonical model spec twice
claim: Agent::new splits the model spec via split_once(':') and then re-parses it via parse_model_spec, which alone carries both halves.
evidence:
  - location: crates/opi-agent/src/agent.rs:583-594
    detail: Two consecutive parses of the same string
criterion_source: null
reproduction: []
confidence: high
status: unverified
```

```yaml
id: INFO-03
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: standards
severity: Info
title: Doc comments cite exact design-spec line numbers
claim: tool_authority doc comments cite volatile design-document line ranges that drift on spec edits.
evidence:
  - location: crates/opi-coding-agent/src/tool_authority.rs:58-59,146-152
    detail: "(spec lines 419-423)" etc.
criterion_source: CLAUDE.md "link volatile facts to their authoritative source instead of copying"
reproduction: []
confidence: high
status: unverified
```

```yaml
id: INFO-04
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: standards
severity: Info
title: gemini/azure/vertex skip the in-adapter capability preflight
claim: Three adapters dispatch stream_prepared without the validate_request_capabilities preflight the other five perform; collection-level validation still covers dispatched calls.
evidence:
  - location: crates/opi-ai/src/gemini.rs:859-898
    detail: stream_prepared goes straight to URL/body construction
  - location: crates/opi-ai/src/anthropic.rs:1272-1274
    detail: Positive pattern present in five other adapters
criterion_source: P17-PRV-004
reproduction: []
confidence: medium
status: unverified
```

---

## 3. Spec Findings

Matt Spec axis verdicts: **67 of 70 criteria met at HEAD; PLT-001, A15, and
the platform legs of RBK-001 partial** (evidence currency only). Non-goals
verified: exactly six workspace crates; `opi-agent` depends only on `opi-ai`
plus generic libs (`uuid` added for the UUIDv7 `RunId`); no
`SharedProvider`/`AgentLoopTurnUpdate`/`AgentHarness`/`HarnessRuntimeConfig`/
`MetadataProvider`/`TraceSink`/`TraceReader` tokens in any production source
(token-level, comment-aware, test-enforced); no exporter/telemetry surface in
core; no alias registry; no new providers.

### 3.1 MAJOR: Three-platform CI acceptance evidence does not cover audit HEAD

**File:** `.github/workflows/ci.yml`
**Lines:** 92--108
**Cause:** The three-platform acceptance evidence cited for P17-PLT-001 /
P17-A15 (run 31798070731) covers exit SHA `40f2e6e` only. Audit HEAD
`a680c5d` is 10 commits later, of which 5 (including two phase-17 runtime
remediations `211aba8` and `a680c5d`) are **unpushed**, so no GitHub Actions
run can exist for the audited code: `gh run list --commit a680c5d` returns
zero runs and the newest run overall is 31803930575 at `877c41f`
(2026-08-14). The phase-17-owned surfaces alone changed by +7,035/-2,005
lines after the CI-evidenced SHA (full crates diff: 149 files,
+29,727/-5,688).
**Impact:** The platform-identity exit criterion is currently unevidenced at
HEAD; Linux/macOS-only regressions in the unpushed range (two `cfg(unix)`
additions exist in it) would be invisible.
**Fix:** Push the 5 commits and run the three-platform `phase17_acceptance`
matrix plus the workspace gates at the final SHA; record the run URL as the
current platform evidence.

```yaml
id: M1
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: spec
severity: Major
title: Three-platform CI acceptance evidence does not cover audit HEAD
claim: The cited three-platform CI evidence (run 31798070731 at 40f2e6e) predates audit HEAD a680c5d by 10 commits including 5 unpushed runtime commits, so PLT-001/A15 and the platform legs of RBK-001 are unevidenced at HEAD.
evidence:
  - location: git ls-remote origin + git log origin/main..HEAD
    detail: origin/main = 877c41f; the 5 commits 211aba8..a680c5d exist on no remote ref, so no CI run can cover them
  - location: gh run view 31798070731
    detail: headSha=40f2e6e, the SHA the ledger pins as FINAL exit evidence
  - location: .github/workflows/ci.yml:92-108
    detail: phase17_acceptance job exists and is platform-neutral at HEAD but has never executed against HEAD
criterion_source: P17-PLT-001; P17-A15; P17-RBK-001
reproduction:
  - gh run list --commit a680c5df13a08d5a2abc48b482a69d1c594f288e
confidence: high
status: unverified
```

### 3.2 MAJOR: `set_model_validated` persists a `model_change` entry its own runtime application then rejects (durable/runtime route divergence)

**File:** `crates/opi-coding-agent/src/harness.rs`
**Lines:** 1988--2011 (persist at 2003--2007, apply at 2008)
**Cause:** The write path orders (1) `try_configure_model` -- which validates
only registry resolvability via `model_info` (`harness.rs:2214-2216` →
`ProviderCollection::registry().resolve`), (2) durable
`session.append_model_change(canonical, input_source)`, and (3)
`apply_agent_model(&model)?`. Extension providers are registered onto the
collection **lookup-only** (`provider_factory.rs:168-172` registers them on
the registry without an auth resolver; only `routes` get `register_route` at
:185-193), so a spec like `ext:ext-model` passes step 1, is persisted at
step 2, and then fails step 3 with `RouteNotDispatchable`
(`provider_collection.rs:498-507` via `Agent::replace_state` validation).
The method returns `Err` to the caller, but the durable entry remains: the
session file records `ext:ext-model` as the branch's latest route while the
live Agent keeps the prior model. Reachable from raw client strings via the
RPC `set_model` command (`rpc.rs:739-760`) and by any SDK caller of the
public method.
**Impact:** Durable/runtime divergence on a normal-path API; the poisoned
entry is then consumed by resume (and triggers M4's panic), or silently
misreports the branch's route in evidence/session tooling.
**Fix:** Validate dispatchability (`validate_dispatchable_route`) inside
`try_configure_model`, or reorder to apply-then-persist; add a regression
test that calls `set_model_validated` with a lookup-only extension provider
and asserts `Err` **plus** an unchanged session (no appended `model_change`).

```yaml
id: M3
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: spec
severity: Major
title: set_model_validated persists a model_change entry its own runtime application rejects
claim: For a registry-resolvable but non-dispatchable (lookup-only extension) provider spec, set_model_validated appends the durable model_change entry before apply_agent_model fails, leaving the session file and live Agent state on different routes while returning Err.
evidence:
  - location: crates/opi-coding-agent/src/harness.rs:1988-2011
    detail: try_configure_model (registry-only validation) -> append_model_change (durable) -> apply_agent_model (dispatchable validation, fails)
  - location: crates/opi-coding-agent/src/provider_factory.rs:168-172
    detail: Extension providers registered lookup-only (registry, no resolver); doc comment states "Lookup-only extension providers"
  - location: crates/opi-ai/src/provider_collection.rs:498-507
    detail: validate_dispatchable_route fails RouteNotDispatchable exactly when resolvers lacks the provider
criterion_source: Task 17.5 DoD ("every new model-change write persist canonical provider:model" with runtime agreement); P17-PRV-002 write side
reproduction: []
confidence: high
status: unverified
```

### 3.3 MAJOR: `apply_recorded_model` panics on a recorded route that is registry-resolvable but not dispatchable

**File:** `crates/opi-coding-agent/src/harness.rs`
**Lines:** 2368--2369
**Cause:** After `try_configure_model(&normalized)` passes (registry resolve
only), `apply_recorded_model` calls
`apply_agent_model(&normalized).expect("recorded model was validated before
application")`. The expected invariant is false for the same
resolvability-vs-dispatchability asymmetry as M3: `apply_agent_model` →
`Agent::replace_state` enforces `validate_dispatchable_route`, which fails
`RouteNotDispatchable` for lookup-only providers. Resuming, forking, or
branch-switching a session whose latest `model_change` names such a route
(produced by M3, or by an extension provider whose resolver disappeared)
panics the process instead of emitting the typed
`CODE_SESSION_RESUME_MODEL_INCOMPATIBLE` diagnostic used one branch earlier.
Reached from `resume_session_id` (:2269), `fork_current_session` (:2458),
`resume_session_branch_tip` (:2569), and the CLI/builder `--resume` paths.
**Impact:** Process crash on session resume for an edge-but-reachable input
class; violates the phase's typed-failure posture (P17-PRV-004) and the 17.8
remediation contract.
**Fix:** Replace the `.expect` with the same warning-diagnostic + keep-CLI-model
path used for incompatible models; add a resume regression test with a
recorded lookup-only route.

```yaml
id: M4
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: spec
severity: Major
title: apply_recorded_model panics via .expect for non-dispatchable recorded routes
claim: Resuming/forking a session whose recorded model_change names a registry-resolvable but resolver-less provider panics at expect("recorded model was validated before application") because validation checks resolvability while application checks dispatchability.
evidence:
  - location: crates/opi-coding-agent/src/harness.rs:2368-2369
    detail: ".expect" after try_configure_model passed; apply_agent_model -> replace_state -> validate_dispatchable_route fails
  - location: crates/opi-coding-agent/src/harness.rs:2352-2366
    detail: The typed CODE_SESSION_RESUME_MODEL_INCOMPATIBLE branch exists one step earlier for incompatible models
criterion_source: P17-PRV-002/P17-PRV-004 typed-failure posture; P17-MIG-001/002 remediation contract
reproduction: []
confidence: high
status: unverified
```

### 3.4 MINOR: AUT-003 "arguments are never consulted" phrasing is falsifiable at HEAD (comments and ledger)

**File:** `crates/opi-coding-agent/src/tool_authority.rs`
**Lines:** 12--16, 321--323, 468--470, 496
**Cause:** The module doc and the inline AUT-003/004 comment state that
`request.arguments` is "never consulted" for permission, but the
`command.execute` capability branch passes `&request.arguments` into
`CommandAuthorizationContext::authorize`, which reads `arguments["backend"]`
to select the adapter whose immutable policy then applies. The design spec
explicitly sanctions this ("Full arguments may be inspected inside this
trusted boundary"; "the permission reference binds the adapter that the
validated arguments will reach"), and the security property (arguments
cannot grant or expand permission; policy digest unchanged) is behaviorally
proven by `phase17_model_content_cannot_expand_effective_policy` and
`phase17_untrusted_sources_cannot_forge_registration_or_grants`. Three
independent finders converged on this.
**Impact:** Misleading security-boundary documentation; the ledger's absolute
phrasing overstates the isolation actually implemented (which is the
design-correct one).
**Fix:** Reword the comments and any live consumer docs to "arguments select
the adapter binding inside the trusted boundary; no permission fact derives
from argument content".

```yaml
id: MIN-08
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: spec
severity: Minor
title: AUT-003 'arguments never consulted' phrasing contradicts the command.execute adapter binding
claim: tool_authority module and inline comments (and the ledger AUT-003 evidence text) claim request.arguments is never consulted, while the command.execute branch reads arguments['backend'] to select the adapter whose policy applies.
evidence:
  - location: crates/opi-coding-agent/src/tool_authority.rs:468-470 vs :496
    detail: Comment 'intentionally NOT consulted' adjacent to command.authorize(&request.arguments, _cancel)
  - location: crates/opi-coding-agent/src/tool_authority.rs:321-323
    detail: arguments.get('backend') feeds resolve_candidate
criterion_source: P17-AUT-003 (phrasing only; behavior is design-conformant)
reproduction: []
confidence: high
status: unverified
```

### 3.5 MINOR: A09-named test's rejection leg was weakened; ledger rows still cite it for the stronger property

**File:** `crates/opi-coding-agent/tests/phase17_product_evidence.rs`
**Lines:** 2063--2075
**Cause:** The pre-`a680c5d` version of
`phase17_complete_run_reconstructs_graph_and_rejects_missing_bindings`
asserted `require_complete()` success and rejected a fabricated
`ActiveSnapshot` binding. At HEAD the only remaining "rejection" assertion is
`ContentDigest::from_hex("").is_err()` (a digest-constructor check unrelated
to the manifest), because `FinalizedManifest` became an opaque validated
wrapper. Three ledger rows (P17-EVD-003, P17-RBK-001 item 2, P17-A09) still
cite this test as proving ActiveSnapshot/config-digest rejection.
**Mitigation:** the constructor-level property remains covered by
`direct_run_never_fabricates_active_snapshot`
(`crates/opi-agent/tests/evidence_contract.rs:754-765`) and manifest
strictness by `ManifestCandidate::validate` tests.
**Impact:** Ledger evidence overstates the cited test; a reader auditing by
citations would believe a leg that no longer exists.
**Fix:** Either restore a manifest-level rejection leg (construct a
`ManifestCandidate` with an `ActiveSnapshot` binding and assert
`validate`/`finalize` fails) or correct the ledger citations to the
contract-level tests.

```yaml
id: MIN-09
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: spec
severity: Minor
title: A09 test's binding-rejection leg weakened to a digest-constructor check while ledger rows still cite the stronger claim
claim: phase17_complete_run_reconstructs_graph_and_rejects_missing_bindings no longer rejects a fabricated ActiveSnapshot binding or missing config digest at HEAD, but three ledger rows still cite it for those rejections.
evidence:
  - location: crates/opi-coding-agent/tests/phase17_product_evidence.rs:2063-2075
    detail: Only ContentDigest::from_hex('').is_err() remains as the rejection leg
  - location: git show a680c5d^:crates/opi-coding-agent/tests/phase17_product_evidence.rs
    detail: Pre-commit version asserted require_complete() and ActiveSnapshot rejection
criterion_source: P17-EVD-003; P17-RBK-001; P17-A09
reproduction: []
confidence: high
status: unverified
```

### 3.6 MINOR: Archived criteria_trace citations and mechanism descriptions are stale relative to HEAD

**File:** `docs/snapshots/phase17/opi-impl-state.json`
**Lines:** criteria_trace rows (various)
**Cause:** Post-exit remediation moved or replaced the cited mechanisms:
`agent.rs:62` for the collection field (actual `:547`), prepare/apply at
`agent_loop.rs:733-761` (actual `:1065-1193`; the atomic apply is now
`std::mem::replace(&mut state, candidate)` at `:1099`), `execute_tool` at
`:1100-1288` (now `preflight_tool` `:1672-1843` + `execute_prepared_tool`
`:1847-1960` after parallel-batch restructuring), `RUN_ID_COUNTER` and
`require_complete` (now UUIDv7 `IdentityAllocator` and
`ManifestCandidate::validate`/`FinalizedManifest`). The underlying
requirements remain substantiated at the new locations (this audit verified
the semantics), and a snapshot ledger is historical -- but the drift is
material for any consumer auditing by citation.
**Impact:** Citation-following verification fails; several rows also
understate HEAD (see INFO-05/INFO-09).
**Fix:** Record a dated addendum in the phase snapshot directory mapping
stale citations to their HEAD locations (do not rewrite the archived ledger).

```yaml
id: MIN-10
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: spec
severity: Minor
title: Archived criteria_trace file:line citations are stale relative to HEAD
claim: Multiple criteria_trace evidence citations point at pre-remediation code locations that no longer match HEAD, though the requirements remain substantiated at the current locations.
evidence:
  - location: ledger P17-OUT-002/NXT-003..006 vs crates/opi-agent/src/agent_loop.rs:1065-1193
    detail: Cited 733-761/753; actual candidate build/validate/mem::replace at 1085-1099
  - location: workspace grep
    detail: Zero matches for RUN_ID_COUNTER and require_complete at HEAD
criterion_source: PRIN-005 (evidence follows the verified implementation)
reproduction: []
confidence: high
status: unverified
```

### 3.7--3.11 Spec-axis Info findings

- **INFO-05 PRV-005 recorded residual is materially narrowed at HEAD** (`provider_factory.rs:186-193` still passes `Static`, but `prepare_call` applies it only when the resolver returned default provenance -- `provider_collection.rs:450-459`, added by `211aba8` -- and all production resolvers report truthful sources: `credential_store.rs:1036-1060, 1417-1490`, `provider_factory.rs:2011-2031`). The five facts remain structurally distinct; criterion met; the recorded residual text no longer describes HEAD.
- **INFO-06 Ledger-documented cross-mode asymmetries are stale (code stronger)**: both "interactive wires no capture" and "RPC does not forward `--trace`" were closed; all modes forward `--trace` (`main.rs:911, 1103, 1259-1264`; CHANGELOG `[Unreleased]`).
- **INFO-07 RPC unconditionally enables the evidence recorder**, so `complete_evidence_required` is always true in RPC even without `--trace` (`rpc.rs:267, 298-301` → `harness.rs:1594-1609`); design-conformant (the RPC `trace` command requires it) but a cross-mode policy divergence worth recording.
- **INFO-08 `terminal_correlation` panics on an empty record set** via documented `.expect` (`opi-coding-agent/src/evidence.rs:653-662`); both production call sites guard it; a typed `EvidenceError::Finalization` would match the fail-closed style.
- **INFO-09 Hermetic-manifest discovery excludes opi-tui/opi-protocol/opi-sandbox** (`phase17_api_audit.rs:897-918, 2604-2613`); latent only (no phase17 tests exist there today).

```yaml
id: INFO-05
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: spec
severity: Info
title: PRV-005 Static-provenance residual narrowed at HEAD by unpushed remediation
claim: Resolver-reported auth provenance now survives prepare_call; only genuinely static resolvers are labeled Static, so the recorded residual overstates the current inaccuracy.
evidence:
  - location: crates/opi-ai/src/provider_collection.rs:450-459
    detail: Route-level Static applied only when auth.provenance == default()
  - location: crates/opi-coding-agent/src/credential_store.rs:1417-1490
    detail: Production resolvers report CredentialStore/OAuth/Environment provenance
criterion_source: P17-PRV-005
reproduction: []
confidence: high
status: unverified
```

```yaml
id: INFO-06
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: spec
severity: Info
title: Ledger cross-mode asymmetry notes are stale; all modes forward --trace at HEAD
claim: The exit-era claims 'interactive wires no capture' and 'RPC does not forward --trace' are false at HEAD; both were closed by post-exit remediation.
evidence:
  - location: crates/opi-coding-agent/src/main.rs:911,1103,1259-1264
    detail: All three mode entries forward cli.trace into evidence capture
criterion_source: P17-MIG-005/P17-A14 (evidence text accuracy)
reproduction: []
confidence: high
status: unverified
```

```yaml
id: INFO-07
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: spec
severity: Info
title: RPC always-on recorder makes complete_evidence_required true without --trace
claim: RPC mode configures an InMemoryEvidenceSink unconditionally, holding RPC to complete-evidence fail-closed semantics that print/interactive modes without --trace are not held to.
evidence:
  - location: crates/opi-coding-agent/src/rpc.rs:267,298-301
    detail: Some(trace_sink) always; in-memory when trace_path is None
  - location: crates/opi-coding-agent/src/harness.rs:1594-1609
    detail: Policy built from build_options.evidence.is_some()
criterion_source: P17-MIG-005
reproduction: []
confidence: high
status: unverified
```

```yaml
id: INFO-08
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: spec
severity: Info
title: terminal_correlation panics on empty record set via documented expect
claim: build_finalized_manifest panics rather than returning the typed EvidenceError when records is empty; guarded at both production call sites today.
evidence:
  - location: crates/opi-coding-agent/src/evidence.rs:653-662
    detail: .expect("manifest correlation requires at least one record")
criterion_source: P17-EVD-003
reproduction: []
confidence: high
status: unverified
```

```yaml
id: INFO-09
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: spec
severity: Info
title: Hermetic-manifest discovery scope excludes three crates
claim: discover_phase17_sources_under scans only opi-agent/opi-ai/opi-coding-agent, so a phase17 test added under opi-tui/opi-protocol/opi-sandbox would escape the manifest guards; latent today.
evidence:
  - location: crates/opi-coding-agent/tests/phase17_api_audit.rs:897-918,2604-2613
    detail: Hard-coded three-crate discovery roots
criterion_source: P17-PLT-002
reproduction: []
confidence: medium
status: unverified
```

---

## 4. Invariant Verification

Matrix built against the **current tightened text** of `docs/opi-spec.md` at
HEAD. Provenance note: PRIN-004, INV-005, and INV-007 were tightened in
`93d75f4`/`aff8875` (2026-08-20) -- six days after phase exit and ~35 minutes
before the remediation commit under audit.

| Invariant | Code evidence | Test coverage | Status |
|-----------|--------------|---------------|--------|
| INV-001 runtime provider resolution | `provider_collection.rs:410-479` prepare_call; per-turn route from applied state (`agent_loop.rs:268-294`); `harness.rs:2021-2046` cross-provider accepted | nxt006 test; `phase17_provider_runtime.rs:240,353`; product evidence | Enforced + tested |
| INV-002 wire behind neutral interfaces | `provider.rs:22-75` sole `stream_prepared`; Bedrock exemption documented | 9 fixture binaries + per_request_auth + provider_trait | Enforced + tested |
| INV-003 exact loop order | `agent_loop.rs:1065-1193` (build→validate→`mem::replace`→stop→queues) | `phase8_hook_contract_order`; A04/A05 phase17 tests | Enforced + tested |
| INV-004 atomic full replacement | `loop_types.rs` NextTurnState; `agent.rs:653-657` `replace_state`; `agent.rs:962` persist-before-settle | `phase17_agent_persists_complete_next_turn_state` | Enforced + tested |
| INV-005 authority before side effects; monotonic order-independent combination | `preflight_tool` chain (`agent_loop.rs:1652-1844`); deny-dominant hook composite (`extension.rs:611-640`); most-restrictive gate (`execution/router.rs:235-258`); sorted-digest policy (`tool_authority.rs:172-226`) | Zero-execution matrix; malicious-content matrix; deny-position tests | Base clause enforced + tested; **tightened permutation/merge verification route untested** (MIN-15); latent first-wins divergence at project-trust seam (MIN-16) |
| INV-006 cancellation/overflow/partial observable | Biased cancel selects; `AgentError::Cancelled`; `retain_strongest_terminal_error`; bounded proxy channel with drop+warn | `phase8_cancellation_contract_*`; failure_rollback cancellation suite | Agent-core paths enforced + tested; bounded-overflow leg weakly tested (MIN-17) |
| INV-007 session durable truth; entry classification; fail-closed unknown-required; interrupted tail | `session.rs:448-542` `read_with_recovery` (version fail-closed :486-494; truncated/corrupt/unknown three-way split) | session_storage/facade suites | Pre-tightening legs enforced + tested; **classification unimplemented** (MIN-13); unsupported-version path untested (MIN-14); durable-pair leg future work (INFO-10) |
| INV-008 finalized evidence identity; direct ≠ ActiveSnapshot | `evidence.rs:483-542` closed binding, `direct()` sole constructor; ManifestCandidate strict validation | binding/manifest suites; product rejections | Enforced + tested |
| PRIN-004 fail closed; most-restrictive merge | Typed errors, no fallback (provider/state/evidence); sorted/structural order-independence at phase seams | failure-precedence suites; `refresh_models_deterministic_ordering` | Fail-closed leg enforced + tested; merge/permutation leg untested (MIN-15) |
| CTRL-001 correlation without exporter | UUIDv7 RunId; IdentityAllocator monotonic sequence; core sinks Noop/InMemory only (test-enforced) | identity/conformance suites | Enforced + tested |
| CTRL-002 offline-verifiable facts | ManifestCandidate correlation/outcome/session/binding/config/route/policy/input/environment/usage/artifacts | graph reconstruction + missing-field rejection | Enforced + tested |
| CTRL-003 redaction before export; no capture by default | Producer-boundary `RedactedValue`; `Agent` default sink `None`; capture only via explicit config | canary suites; default-no-evidence test | Enforced + tested |

### 4.1 MINOR: In-band provider stream `Error` terminal is converted into a normally completed turn

**File:** `crates/opi-agent/src/agent_loop.rs`
**Lines:** 1277--1278 (seam), 415--429 (consumption), 900--975 (retry gate)
**Cause:** `process_stream_event` returns `Some(message.clone())` for both
`Done` and `Error` terminals, so an in-band failure event (every adapter maps
these: Anthropic SSE `error`, Gemini/OpenAI/Bedrock stream errors) is pushed
into durable context as a complete assistant message, the turn lifecycle
proceeds normally, and the run finishes `Ok`. The retry logic gates on
`Err(e) if e.is_retryable()` only, so a retryable-class in-band error (e.g.
overload delivered as an SSE error event) is neither retried nor surfaced as
a run failure or diagnostic.
**Impact:** A provider failure class reaches users/evidence as a normal
completed turn; FAL-003's "not converted into success" posture is weakened
for this delivery shape.
**Fix:** Distinguish the `Error` terminal at the seam (e.g. return a typed
flag or synthesize the retryable/failed `Err` path) and pin with a test
driving an in-band error event through a mock provider.

```yaml
id: MIN-12
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: invariants
severity: Minor
title: In-band stream Error terminal treated identically to Done
claim: An AssistantStreamEvent::Error terminal is returned as a complete assistant message, persisted to context, and the run finishes Ok with no retry classification or failure diagnostic.
evidence:
  - location: crates/opi-agent/src/agent_loop.rs:1277-1278
    detail: "Done { message, .. } => Some(message.clone()), Error { message, .. } => Some(message.clone())"
  - location: crates/opi-agent/src/agent_loop.rs:900-975
    detail: Retry gate handles Err only; an Ok(Error-terminal) event never reaches it
criterion_source: P17-FAL-003; INV-006
reproduction: []
confidence: high
status: unverified
```

### 4.2 MINOR: INV-007 tightened required-vs-ignorable entry classification is unimplemented

**File:** `crates/opi-agent/src/session.rs`
**Lines:** 18--23, 214--229, 516--533
**Cause:** `SessionReader` treats every unrecognized `type` tag as skippable
and non-fatal ("never fatal -- this is the forward-compatibility path"), and
`SessionEntry` carries no required/ignorable classification, so a future
required entry would be silently skipped while resume reports success -- the
outcome the tightened INV-007 forbids. The tightening (`aff8875`) postdates
phase exit; the design doc scoped 17.8's INV-007 responsibility to
"preserve existing session reconstruction and crash-recovery semantics",
and the current additive-entry policy is internally consistent and honestly
documented (`SESSION_FORMAT_POLICY`).
**Impact:** Future-work clause at a phase-owned boundary; route it to an
implementing phase rather than leaving it implicit.
**Fix:** Add an envelope classification (required vs explicitly-ignorable)
and a fail-closed unknown-required path with fixtures, in the phase that
next touches the session format.

```yaml
id: MIN-13
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: invariants
severity: Minor
title: INV-007 required-vs-ignorable session entry classification unimplemented
claim: Unknown session entry types are implicitly ignorable and never fatal, with no envelope field classifying entries as required, so the tightened fail-closed-on-unknown-required clause is unimplemented.
evidence:
  - location: crates/opi-agent/src/session.rs:18-23
    detail: Module doc: unknown type tag "skipped and counted ... never fatal"
  - location: docs/opi-spec.md:425
    detail: Tightened INV-007 requires fail-closed on unknown required entries
criterion_source: INV-007 (tightened post-exit)
reproduction: []
confidence: high
status: unverified
```

### 4.3 MINOR: Unsupported-session-version fail-closed path has zero test coverage

**File:** `crates/opi-agent/src/session.rs`
**Lines:** 486--494
**Cause:** The enforcing code exists (`header.version != FORMAT_VERSION` →
typed error) but no test writes a version-2/0 header and asserts rejection;
the only version assertions are round-trip equality checks.
**Impact:** One-fixture gap on a fail-closed leg whose code already exists.
**Fix:** Add the fixture.

```yaml
id: MIN-14
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: invariants
severity: Minor
title: Unsupported-session-version rejection untested
claim: No workspace test asserts the typed error for a session header with an unsupported format version.
evidence:
  - location: crates/opi-agent/src/session.rs:486-494
    detail: Enforcing branch with no matching test
criterion_source: INV-007 verification route "unsupported-version diagnostics"
reproduction: []
confidence: high
status: unverified
```

### 4.4 MINOR: No registration-order permutation / authority-merge matrix tests (tightened PRIN-004/INV-005 verification route)

**Cause:** Workspace grep finds no test registering authority contributors in
two orders and asserting decision invariance. Deny-dominance is covered
positionally (`extensions.rs:583/811/1362`), and the combination semantics
are order-independent by construction at the phase-17 seams (deny
short-circuit `extension.rs:611-640`; most-restrictive gate
`execution/router.rs:235-258`; sorted-digest policy `tool_authority.rs:182-213`),
so this is a test-route gap against the tightened clause, not an enforcement
gap. The clause postdates phase exit and no phase has been tasked with it.
**Fix:** Add a permutation matrix test over hooks/extensions/adapter gates.

```yaml
id: MIN-15
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: invariants
severity: Minor
title: No registration-order permutation or authority-merge matrix tests
claim: No test permutes the registration order of authority contributors and asserts the combined decision is invariant, the verification route the tightened PRIN-004/INV-005 demand.
evidence:
  - location: workspace grep permutat|registration_order|order_independen|most_restrictive|monotonic
    detail: Only provider-id sorting and hook call-order traces; none permute authority decisions
criterion_source: PRIN-004; INV-005 (tightened post-exit)
reproduction: []
confidence: high
status: unverified
```

### 4.5 MINOR: Project-trust resolver votes resolve first-wins in registration order, diverging from tightened PRIN-004

**File:** `crates/opi-coding-agent/src/project_trust.rs`
**Lines:** 478--484
**Cause:** Conflicting decided votes (Trusted vs Untrusted) from two
registered resolvers resolve first-registered-wins, not
most-restrictive; permuting registration order flips the outcome. Pinned as
intended by `project_trust_store.rs:789-809`. Latent only: the standard CLI
constructs an empty resolver registry (`main.rs:334-335`). Phase 15 seam,
untouched by Phase 17; the tightening postdates both.
**Fix:** Reconcile the seam (most-restrictive conflict rule) or record an
explicit spec exception in a later phase.

```yaml
id: MIN-16
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: invariants
severity: Minor
title: Project-trust resolver conflicts resolve first-wins, not most-restrictive
claim: Two registered project-trust resolvers with conflicting decided votes resolve in registration order, so the outcome depends on registration order; latent because the standard CLI registers none.
evidence:
  - location: crates/opi-coding-agent/src/project_trust.rs:478-484
    detail: "the first Trusted/Untrusted wins" in deterministic registration order
  - location: crates/opi-coding-agent/tests/project_trust_store.rs:789-809
    detail: Behavior pinned as intended
criterion_source: PRIN-004 (tightened)
reproduction: []
confidence: high
status: unverified
```

### 4.6 MINOR: Bounded-proxy overflow test never saturates the channel; ledger cites it as the overflow-class evidence

**File:** `crates/opi-agent/tests/streaming_proxy.rs`
**Lines:** 488--507
**Cause:** `bounded_event_channel_capacity_respected` uses capacity 2,
submits one command, and asserts `messages.len() >= 2`; it never fills the
channel or observes a drop, and the drop path itself surfaces only a
server-side `tracing::warn!` (`streaming_proxy.rs:251-257`) invisible to the
SDK/RPC consumer. The ledger FAL-003 row names this test as the bounded
overflow evidence, overstating it.
**Fix:** Add a saturation test asserting either delivery-with-backpressure or
a consumer-visible overflow signal.

```yaml
id: MIN-17
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: invariants
severity: Minor
title: Overflow-observability test does not saturate the bounded channel
claim: The cited bounded-channel test never fills the channel nor observes a drop, so INV-006 overflow observability over the proxy channel is untested and the ledger overstates the evidence.
evidence:
  - location: crates/opi-agent/tests/streaming_proxy.rs:488-507
    detail: capacity 2, one command, len>=2 assertion only
  - location: crates/opi-agent/src/streaming_proxy.rs:251-257
    detail: Drop path emits only a server-side tracing::warn
criterion_source: INV-006; P17-FAL-003
reproduction: []
confidence: high
status: unverified
```

### 4.7 MINOR: Shared sink conformance contract does not pin emit-before-setup; oracle and product adapter diverge

**File:** `crates/opi-agent/src/evidence.rs`
**Lines:** 2405--2424 vs `crates/opi-coding-agent/src/evidence.rs:225-234`
**Cause:** `InMemoryEvidenceSink::emit` gates only on `finalized` (accepts
records before any `setup`), while `FileEvidenceSink::emit` fails closed
("evidence sink used before setup") and marks the lifecycle failed. The one
shared conformance contract exercises only the setup→emit→finalize happy
path, so the divergence is unpinned.
**Impact:** A harness lifecycle bug would be invisible to the oracle but fail
in production; EVD-011's "one applicable lifecycle/failure conformance
contract" is incomplete. Production wiring always sets up first, so no
user-visible behavior today.
**Fix:** Extend the shared contract with an emit-before-setup leg (expect the
file adapter's typed failure; align the in-memory oracle to the same
fail-closed phase gate).

```yaml
id: MIN-18
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: invariants
severity: Minor
title: Emit-before-setup behavior diverges between in-memory oracle and file adapter, unpinned by the shared contract
claim: InMemoryEvidenceSink accepts emit before setup while FileEvidenceSink fails closed, and the shared conformance contract does not cover this lifecycle leg.
evidence:
  - location: crates/opi-agent/src/evidence.rs:2405-2424
    detail: emit checks only the finalized flag
  - location: crates/opi-coding-agent/src/evidence.rs:225-234
    detail: emit requires Active|FailedActive phase else typed failure + mark_failure
criterion_source: P17-EVD-011
reproduction: []
confidence: high
status: unverified
```

### 4.8 Invariant Info findings

- **INFO-10** Tightened INV-007's "committed prefix + immutable Runtime Input Binding as one validated durable pair" is realized as session file plus per-run manifest binding; resume derives from session entries only and records a fresh `DirectRuntimeInput` digest. No dual independently-mutable durable owner exists today; the derive-from-the-same-pair leg is future work aimed at the Promotion-Controller world.
- **INFO-11** `InMemoryEvidenceSink::setup` unconditionally resets failure/records/manifest on re-setup (`evidence.rs:2390-2403`), unlike the file adapter's fail-closed re-setup rejection; within a single lifecycle EVD-008 holds in both.

```yaml
id: INFO-10
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: invariants
severity: Info
title: Durable-prefix-plus-binding pair unrealized as one validated unit at resume
claim: Resume/fork derive from session entries alone and re-bind a fresh DirectRuntimeInput per run; the tightened same-pair derivation is future work, with no dual mutable durable owner existing today.
evidence:
  - location: crates/opi-coding-agent/src/harness.rs:2560-2569
    detail: Resume derives context/route from session entries only
criterion_source: INV-007 (tightened post-exit)
reproduction: []
confidence: medium
status: unverified
```

```yaml
id: INFO-11
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: invariants
severity: Info
title: InMemoryEvidenceSink setup resets failure state on re-setup
claim: The core conformance oracle clears has_failure/records/manifest when setup is re-invoked, unlike the file adapter's fail-closed rejection; can erase observability of an earlier lifecycle on the same sink.
evidence:
  - location: crates/opi-agent/src/evidence.rs:2390-2403
    detail: Unconditional reset then re-bind
criterion_source: P17-EVD-008 (adjacent)
reproduction: []
confidence: medium
status: unverified
```

---

## 5. Test-Quality Findings

Overall: the suites assert real behavior (execution counters with exact
counts, byte-identity comparisons, digests, event sequences, wire-level
assertions) and are hermetic (MockProvider, temp dirs, no live endpoints --
independently re-verified by the security finder's endpoint grep). The
findings are assertion-strength gaps and citation mismatches.

### 5.1 MINOR: Stale-allow reauthorization leg untested; ledger citation stale

**File:** `crates/opi-agent/tests/tool_authority.rs`
**Lines:** 412--466
**Cause:** Every stale-path fixture uses an authorizer whose generation is
fixed-stale on every call, so the tests pass whether or not the
reauthorize-once loop runs; no test provides stale-first/fresh-second to
prove the recovery direction, and no authorize-call-count assertion exists.
The ledger's P17-A12/OUT-003 evidence cites `evidence_runtime.rs:319`, stale
at HEAD. The production seam exists (`agent_loop.rs:1492-1600`, `for attempt
in 0..2u8`).
**Fix:** Add a stale-then-fresh authorizer fixture asserting exactly two
authorize calls and one execution.

```yaml
id: MIN-19
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: test-quality
severity: Minor
title: Stale-allow reauthorization recovery leg untested
claim: No test proves the reauthorize-once recovery direction (stale then fresh) or the authorize-call count; existing stale tests cannot distinguish deny-on-first-stale from reauthorize-then-deny.
evidence:
  - location: crates/opi-agent/tests/tool_authority.rs:412-466
    detail: Fixed-stale authorizer; count==0 assertion only
  - location: crates/opi-agent/src/agent_loop.rs:1492-1600
    detail: Production reauthorize-once loop with comment, untested recovery direction
criterion_source: P17-A12; P17-OUT-003
reproduction: []
confidence: high
status: unverified
```

### 5.2 MINOR: A05 state-preservation tests do not verify inference/model fields against non-default priors

**File:** `crates/opi-agent/tests/hooks_queues.rs`
**Lines:** 1452--1489, 1536--1574
**Cause:** The two prepare-failure tests assert preservation only via
`messages_snapshot().len() == 1` with default `InferenceConfig` priors, so a
failure-path bug resetting inference or model_selection to defaults would
pass. Sibling tests in the same file (`:1627-1684, 1745-1813, 1977-2043`) do
set non-default baselines and assert full-field preservation, showing the
intended pattern.
**Fix:** Add the baseline fields + full-field assertions to both tests.

```yaml
id: MIN-20
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: test-quality
severity: Minor
title: Prepare-failure preservation tests invisible to inference/model resets
claim: phase17_failed_prepare... and phase17_invalid_prepare_candidate... assert state preservation only via message count with default priors, so inference/model_selection resets would pass.
evidence:
  - location: crates/opi-agent/tests/hooks_queues.rs:1452-1489,1536-1574
    detail: len==1 assertions, InferenceConfig::default(), no model/inference checks
criterion_source: P17-A05; P17-OUT-002
reproduction: []
confidence: high
status: unverified
```

### 5.3 MINOR: `binding_variants` "not normalizable" assertion is a test-local classifier tautology

**File:** `crates/opi-agent/tests/evidence_contract.rs`
**Lines:** 737--751
**Cause:** The not-normalizable half of the cited test maps the two enum
variants to 0/1 via a test-local `kind()` and asserts they differ -- an
assertion that passes for any two distinct variants and would survive a serde
collapse of both bindings to identical JSON. Sibling tests in the same file
demonstrate the real serialization-distinctness pattern (render +
`BTreeSet`).
**Fix:** Replace with the render/uniqueness pattern used by
`measurement_origins_are_distinct`.

```yaml
id: MIN-21
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: test-quality
severity: Minor
title: binding_variants not-normalizable assertion is tautological
claim: The cited 'not normalizable' evidence rests on a test-local classifier asserting two enum variants differ, not on serialization distinctness.
evidence:
  - location: crates/opi-agent/tests/evidence_contract.rs:737-751
    detail: fn kind(&RuntimeInputBinding)->u8 + assert_ne!
  - location: crates/opi-agent/tests/evidence_contract.rs:789-806
    detail: Sibling real pattern (render + BTreeSet uniqueness)
criterion_source: P17-EVD-003
reproduction: []
confidence: high
status: unverified
```

### 5.4--5.8 Test-quality Info findings

- **INFO-12** A04 context leg is a length proxy (`observed_context_len >= 3`), not content; other four fields asserted by value (`hooks_queues.rs:1405-1412`).
- **INFO-13** Ledger "exactly ONE Provider record" for A03 misstates the assertion: the test proves one distinct Provider call identity via `HashSet`, with multiple lifecycle records sharing it by design (`evidence_runtime.rs:1050-1062`).
- **INFO-14** `parent_call_link_correlates_retry_to_origin` re-asserts values the test itself constructed; the load-bearing behavioral proof is `retry_emits_retry_record_parented_to_provider_call` (`evidence_contract.rs:577-593`).
- **INFO-15** PRV-006 ledger row claims "five adapters" via wiremock; `per_request_auth.rs` drives four (complementary boundary coverage exists in the gemini/azure/vertex/bedrock fixture suites).
- **INFO-16** The 17.9 artifact bundle transcribes hardcoded values into files whose RUN_SUMMARY rows are marked "verified" (`tool-execution-counts.json` literal zeros, `provider-assertion.json` literal `calls: 2`, unconditional `exit-code.txt "0"`); the genuine verification lives in the in-test assertions, so the artifact rows overstate their own provenance (`phase17_cross_mode.rs:797-838` vs the digest-verified pattern in `phase17_legacy_migration.rs:553-565`).

```yaml
id: INFO-12
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: test-quality
severity: Info
title: A04 context observation asserted by length proxy
claim: The context leg of the five-field stop-observation test asserts only observed_context_len >= 3 rather than the prepared message content.
evidence:
  - location: crates/opi-agent/tests/hooks_queues.rs:1405-1412
    detail: Length assertion while the other four fields are exact-value
criterion_source: P17-A04
reproduction: []
confidence: medium
status: unverified
```

```yaml
id: INFO-13
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: test-quality
severity: Info
title: Ledger 'exactly ONE Provider record' wording mismatches the one-call-identity assertion
claim: The A03/PRV-003 ledger text says exactly one Provider record, but the test asserts one distinct Provider call identity with multiple lifecycle records sharing it.
evidence:
  - location: crates/opi-agent/tests/evidence_runtime.rs:1050-1062
    detail: HashSet of call ids len==1 with explicit multi-record comment
criterion_source: P17-PRV-003
reproduction: []
confidence: high
status: unverified
```

```yaml
id: INFO-14
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: test-quality
severity: Info
title: parent_call_link test re-asserts constructed values
claim: The EVD-001-cited contract test asserts only fields the test itself constructed; the behavioral proof lives in the runtime test.
evidence:
  - location: crates/opi-agent/tests/evidence_contract.rs:577-593
    detail: Construct-then-assert with no production path exercised
criterion_source: P17-EVD-001
reproduction: []
confidence: high
status: unverified
```

```yaml
id: INFO-15
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: test-quality
severity: Info
title: PRV-006 ledger overcounts wiremock adapter coverage
claim: The ledger says five adapters are driven via wiremock in per_request_auth.rs; the file drives four, with complementary coverage in fixture suites.
evidence:
  - location: crates/opi-ai/tests/per_request_auth.rs:88-349
    detail: Anthropic x2, OpenAiChat x2, OpenAiResponses x2, Codex x1 + one collection-level test
criterion_source: P17-PRV-006
reproduction: []
confidence: high
status: unverified
```

```yaml
id: INFO-16
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: test-quality
severity: Info
title: 17.9 artifact bundle rows marked verified over transcribed values
claim: tool-execution-counts.json, provider-assertion.json, and exit-code.txt contain hardcoded values while RUN_SUMMARY marks their rows verified; real verification lives in in-test assertions.
evidence:
  - location: crates/opi-coding-agent/tests/phase17_cross_mode.rs:797-838
    detail: Literal json! zeros / "calls": 2 / "0\n" written before the test finishes
criterion_source: P17-MIG-005 artifact-truthfulness convention
reproduction: []
confidence: high
status: unverified
```

---

## 6. Integration Findings

### 6.1 MINOR: Eagerly-built extra dispatch routes with invalid non-secret config are dropped silently

**File:** `crates/opi-coding-agent/src/provider_factory.rs`
**Lines:** 1544--1575
**Cause:** Route construction failures for non-secret reasons (bad proxy,
malformed profile) skip the provider (`if let Ok(route) = route`, doc: "skipped
silently"); `ProviderBundle.diagnostics` carries only active-provider
diagnostics. A later model switch then fails as "unknown model" and read-side
bare normalization treats the provider as absent, with nothing pointing at the
broken config.
**Fix:** Surface a startup diagnostic per dropped route (non-secret reason).

```yaml
id: MIN-22
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: integration
severity: Minor
title: Invalid extra-route configs dropped without diagnostic
claim: A provider whose eager route construction fails for a non-secret config reason is silently absent from the dispatch collection with no diagnostic, degrading later failures to unknown-model errors.
evidence:
  - location: crates/opi-coding-agent/src/provider_factory.rs:1544-1575
    detail: "if let Ok(route)" drops at three sites; doc comment admits silent skip
criterion_source: P17-PRV-001/P17-PRV-004 (adjacent)
reproduction: []
confidence: high
status: unverified
```

### 6.2 MINOR: Public infallible SDK seams panic on invalid model input where pre-phase behavior failed typed at prompt time

**File:** `crates/opi-coding-agent/src/harness.rs`
**Lines:** 1718--1728, 1901--1905
**Cause:** `CodingHarness::builder(...).build()` and `set_model` now panic
via `.expect` when the model spec is not dispatchable (the underlying
`Agent::new`/`replace_state` are typed `Result`s whose errors the harness
unwraps). At the pre-phase commit `4c4a840` the same calls constructed
successfully and failed typed at prompt time. Production CLI paths
pre-validate, so this is embedder-facing.
**Fix:** Return typed errors from the builder/`set_model` (0.x breaking change
is permitted and belongs in `[Unreleased]`), or document the panic contract
explicitly.

```yaml
id: MIN-23
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: integration
severity: Minor
title: Embedder-facing builder/set_model panic on non-dispatchable model
claim: CodingHarness builder build() and set_model panic via expect for invalid/non-dispatchable model input, where pre-Phase-17 the same calls constructed and failed typed at prompt time.
evidence:
  - location: crates/opi-coding-agent/src/harness.rs:1718-1728,1901-1905
    detail: expect("startup model selection must resolve to a dispatchable route") / expect("model change must keep a dispatchable route")
criterion_source: 17.5 DoD constructor-surface change
reproduction: []
confidence: medium
status: unverified
```

---

## 7. Security / Redaction Findings

Dimension result: **P17-FAL-004, P17-A10, and P17-EVD-005 are met.** Secrets
were traced end to end (env, config, keyring, OAuth, AWS profile /
credential-process → `ResolvedAuth`/`PreparedProviderCall` → wire): every
secret-bearing type has a manual redacting `Debug`; all 14 production
`expose_secret` sites are wire uses; SigV4 canonicalization never prints the
key; provider 5xx bodies are never read and 401/403 bodies are dropped
("a proxy may echo the submitted credential"); query-string/credentialed-URL
secrets scrubbed; `FileEvidenceSink` uses exclusive-create + temp-rename-sync
publication (no TOCTOU); the phase17 test trees contain no live endpoints or
credentials. Four hardening Infos:

- **INFO-17** Adapter stderr lines logged verbatim at `tracing::debug!` (`adapter_host.rs:228-238`) unlike the content-free execution-backend drain; no subscriber is installed in shipped binaries today.
- **INFO-18** AWS env secrets transit as plain non-zeroizing `String`s before `SecretString` wrapping (`bedrock/credentials.rs:728-741`; `provider_factory.rs:1960-1967`).
- **INFO-19** Bedrock wire path materializes the session token and signed Authorization header as plain owned `String`s (`bedrock/mod.rs:310-314, 334-337`).
- **INFO-20** `ProxyConfig`/`HttpClient` derive `Debug` over the raw proxy URL while `redact_proxy_credentials` (`http.rs:295-309`) has zero production callers (verified by grep); no current `{:?}` print path exists.

```yaml
id: INFO-17
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: security
severity: Info
title: Adapter stderr logged verbatim at debug level
claim: adapter_host drains adapter-process stderr into tracing::debug! unredacted; an embedder-installed subscriber would log adapter-controlled content verbatim.
evidence:
  - location: crates/opi-coding-agent/src/adapter_host.rs:228-238
    detail: tracing::debug!(target: "adapter_stderr", %line)
criterion_source: P17-FAL-004 (adjacent hardening)
reproduction: []
confidence: high
status: unverified
```

```yaml
id: INFO-18
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: security
severity: Info
title: AWS env secrets transit as plain Strings before secrecy wrapping
claim: AWS_SECRET_ACCESS_KEY/AWS_SESSION_TOKEN are read into plain non-zeroizing Strings before SecretString wrapping; memory-hygiene defense-in-depth only, no output path exposes them.
evidence:
  - location: crates/opi-ai/src/bedrock/credentials.rs:728-741
    detail: credentials_from_env returns Option<String>
criterion_source: null
reproduction: []
confidence: high
status: unverified
```

```yaml
id: INFO-19
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: security
severity: Info
title: Bedrock wire path copies session token and Authorization header as plain Strings
claim: token.expose_secret().to_owned() and signed.authorization.expose_secret().to_owned() create non-zeroized plaintext copies that outlive the expose call; wire-only.
evidence:
  - location: crates/opi-ai/src/bedrock/mod.rs:310-314,334-337
    detail: Plain owned Strings into route headers
criterion_source: null
reproduction: []
confidence: high
status: unverified
```

```yaml
id: INFO-20
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: security
severity: Info
title: ProxyConfig/HttpClient derived Debug over raw proxy URL; redact helper unused in production
claim: Derived Debug on ProxyConfig/HttpClient would print an embedded proxy password, and the purpose-built redact_proxy_credentials has no production caller; latent footgun only.
evidence:
  - location: crates/opi-ai/src/http.rs:91-97,116-122,295-309
    detail: Derived Debug + unused redactor
criterion_source: null
reproduction: []
confidence: high
status: unverified
```

---

## 8. Residuals Findings

### 8.1 MAJOR: Post-exit test rewrite at audit_head has no CI or recorded full-suite validation

**Cause:** `a680c5d` ("fix: close phase 17 remediation gaps") rewrote the
phase17 suites at ~8k inserted lines (33 test files;
`phase17_api_audit.rs` alone 374 → 3,223 lines with a hand-written Rust
lexer) **and** rewrote production source (~15k lines: `evidence.rs` +1738,
`agent_loop.rs` +1497, `agent.rs` +770, `bedrock/credentials.rs` +1308,
`harness.rs` +1667...), so code and tests changed together after all recorded
green evidence. Zero CI runs exist for it (see M1), and no recorded
full-suite local run predates this audit. Every criteria_trace "ran green"
claim and the PLT-001 three-platform bundle
(`target/opi-artifacts/phase17-exit/`, dated 2026-08-14) attach to the
pre-rewrite tree.
**Mitigation produced by this audit** (not previously recorded): 621 tests
across 26 phase17-focused binaries green at HEAD on Windows, `cargo fmt
--check --all` green, clippy `-D warnings` green on the three crates' lib
targets, and `opi-doc-check.py` PASS. The workspace-wide gates
(`--all-targets` test, doc tests, rustdoc) and the Linux/macOS legs remain
unrecorded at HEAD.
**Fix:** Run and record the six source-ordered gates plus the three-platform
matrix at the final SHA; treat ledger "ran green" rows as historical until
then.

```yaml
id: M2
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: residuals
severity: Major
title: Post-exit rewrite has no CI or recorded full-suite validation at audit_head
claim: a680c5d changed ~8k test and ~15k production lines together with zero CI runs and no recorded full-suite validation, so all ledger green-evidence claims attach to the pre-rewrite tree rather than audit_head.
evidence:
  - location: git show --stat a680c5d
    detail: 127 files; phase17 suites +8,022/-1,212; src rewrites incl. evidence.rs +1738 and agent_loop.rs +1497
  - location: gh run list --commit a680c5d
    detail: Zero runs; newest run 31803930575 at 877c41f (2026-08-14)
  - location: target/opi-artifacts/phase17-exit/ mtimes
    detail: Exit evidence bundle dated 2026-08-14, pre-rewrite
criterion_source: P17-PLT-001; P17-A15; PRIN-005
reproduction: []
confidence: high
status: unverified
```

### 8.2 MINOR: Failed-run rollback discards successful turns; harness retry re-executes their side effects

**File:** `crates/opi-agent/src/agent.rs`
**Lines:** 950--963
**Cause:** On any run error other than `PartialSideEffect`/`CleanupUnknown`,
`run_with_token` skips `self.state = run.state`, discarding not just the
failing turn but all successful turns of that run. The harness's
`retry_last_prompt` (used after `CredentialNeeded` is resolved via
interactive login) performs no rewind and re-runs from the user message, so
successful earlier turns' tool side effects re-execute. The comment marks
this as the intentional pre-existing rollback contract; `prepare_call`
re-evaluates auth between turns, enabling the mid-run trigger.
**Impact:** Idempotency risk on retry after a mid-run credential failure; no
silent success, and durable session state is untouched (rollback direction).
**Fix:** Persist successful-turn prefix on run failure (or rewind to the last
completed turn before retry) and pin with a test.

```yaml
id: MIN-24
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: residuals
severity: Minor
title: Failed-run rollback discards successful turns; retry re-executes side effects
claim: Agent::run_with_token discards the whole run state on ordinary failures, so harness retry_last_prompt re-executes successful earlier turns including their tool side effects.
evidence:
  - location: crates/opi-agent/src/agent.rs:950-963
    detail: "if run.error.is_none() || matches!(PartialSideEffect|CleanupUnknown) { self.state = run.state; }"
  - location: crates/opi-coding-agent/src/harness.rs:2651-2668
    detail: retry_last_prompt performs no rewind before re-running
criterion_source: null
reproduction: []
confidence: medium
status: unverified
```

### 8.3--8.10 Residual Info findings

- **INFO-21** Blocking per-record file IO (write+flush; fsync at finalize) executes synchronously inside the async agent loop (`opi-coding-agent/src/evidence.rs:259-263, 343`; `agent_loop.rs:2108`); each record stalls a tokio worker on disk.
- **INFO-22** `FileEvidenceState.completed_dirs` accumulates unboundedly across runs for the sink's lifetime (`evidence.rs:90, 390`).
- **INFO-23** Vestigial `assistant_content` accumulator written but never read (`agent_loop.rs:263, 388, 416`); the loop uses the provider's complete Done/Error payload.
- **INFO-24** Ledger EVD-001/EVD-003 evidence names mechanisms that no longer exist (`RUN_ID_COUNTER`, `require_complete`); replaced by UUIDv7 allocator and `ManifestCandidate::validate` (criterion still met).
- **INFO-25** `AuthFailed` (static-route 401/403 rejection) does not mark the prepared call credential-terminal, so a non-retry-gated caller could re-dispatch a rejected frozen credential (`provider_collection.rs:727-734`); no current production path does.
- **INFO-26** `AuthProvenance` default value doubles as the "unreported" sentinel, so a resolver deliberately reporting the default cannot be distinguished from one that never set it (`provider_collection.rs:453-459`); no in-repo resolver hits the mislabel path.
- **INFO-27** RPC `session_info` `tree_read_error` embeds the raw read error (usually containing the session path) unredacted while sibling fields use Summary redaction (`rpc.rs:956-960`); pre-existing Phase 13.5 surface, untouched by Phase 17.
- **INFO-28** Ledger cross-mode/hermeticity rows describe the older, weaker tests; HEAD's are stronger (two dispatches per mode through the production TUI loop, interactive in-memory capture with kinds equality, RPC durable `--trace` root, 26-file manifest) -- conservative drift, code stronger than recorded.

```yaml
id: INFO-21
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: residuals
severity: Info
title: Blocking evidence file IO on the async loop
claim: FileEvidenceSink emit performs mutex-guarded write+flush per record and fsync+rename at finalize, called synchronously from the async agent loop.
evidence:
  - location: crates/opi-coding-agent/src/evidence.rs:259-263,343
    detail: Sync IO while holding the state mutex
criterion_source: null
reproduction: []
confidence: medium
status: unverified
```

```yaml
id: INFO-22
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: residuals
severity: Info
title: Unbounded completed-run directory list on FileEvidenceSink
claim: completed_dirs grows one PathBuf per finalized run for the sink lifetime with no pruning.
evidence:
  - location: crates/opi-coding-agent/src/evidence.rs:90,390
    detail: Vec<PathBuf> appended in finalize_run, cloned by completed_run_dirs()
criterion_source: null
reproduction: []
confidence: medium
status: unverified
```

```yaml
id: INFO-23
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: residuals
severity: Info
title: Vestigial assistant_content accumulator never read
claim: The per-attempt text/tool accumulator is cleared and appended but never read; the loop consumes the provider's complete terminal payload.
evidence:
  - location: crates/opi-agent/src/agent_loop.rs:263,388,416-419
    detail: Three occurrences, no read site; comment states the complete-payload choice
criterion_source: null
reproduction: []
confidence: high
status: unverified
```

```yaml
id: INFO-24
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: residuals
severity: Info
title: Ledger evidence cites mechanisms removed at HEAD
claim: RUN_ID_COUNTER and require_complete no longer exist; replaced by UUIDv7 IdentityAllocator and ManifestCandidate::validate with equal-or-stronger semantics.
evidence:
  - location: workspace grep
    detail: Zero matches for either symbol at HEAD
criterion_source: P17-EVD-001; P17-EVD-003 (citation accuracy)
reproduction: []
confidence: high
status: unverified
```

```yaml
id: INFO-25
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: residuals
severity: Info
title: AuthFailed does not mark the prepared call credential-terminal
claim: Only CredentialNeeded/CredentialRevoked set credential_terminal, so a stream-time AuthFailed leaves re-dispatch of the rejected frozen credential possible for a caller that ignores is_retryable.
evidence:
  - location: crates/opi-ai/src/provider_collection.rs:727-734
    detail: poll_next matches only the two credential variants
criterion_source: P17-PRV-004 (adjacent)
reproduction: []
confidence: high
status: unverified
```

```yaml
id: INFO-26
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: residuals
severity: Info
title: AuthProvenance default doubles as the unreported sentinel
claim: prepare_call overwrites an exactly-default provenance with the route registration source, conflating a deliberate default report with no report; no in-repo resolver hits the path.
evidence:
  - location: crates/opi-ai/src/provider_collection.rs:453-459
    detail: "if auth.provenance == AuthProvenance::default() { ... }"
criterion_source: P17-PRV-005 (adjacent)
reproduction: []
confidence: high
status: unverified
```

```yaml
id: INFO-27
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: residuals
severity: Info
title: RPC tree_read_error field carries unredacted error text
claim: The session_info RPC response embeds the raw session-read error while sibling fields use Summary redaction; pre-existing Phase 13.5 surface.
evidence:
  - location: crates/opi-coding-agent/src/rpc.rs:956-960
    detail: format!("{e}") into the response
criterion_source: P17-FAL-004 (adjacent)
reproduction: []
confidence: high
status: unverified
```

```yaml
id: INFO-28
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: residuals
severity: Info
title: Ledger cross-mode/hermeticity rows describe weaker pre-rewrite tests
claim: MIG-005/A14/PLT-002 ledger text cites the older single-dispatch/8-file-scan tests; HEAD's are materially stronger.
evidence:
  - location: crates/opi-coding-agent/tests/phase17_cross_mode.rs:456-876,1370-1454
    detail: Two dispatches per mode, interactive capture kinds equality, RPC durable trace root
criterion_source: P17-MIG-005; P17-A14; P17-PLT-002
reproduction: []
confidence: high
status: unverified
```

---

## 9. Minimum-change Conformance

All nine tasks carry the full four-note standardized trace (reuse_search,
placement, surface_necessity, simplification_ceiling); statuses compare the
admitted trace with the complete implementation at `audit_head`.

| Task | Scenario/source | Reuse | Placement | Surface | Production slice | Ceiling/trigger | Status |
|------|-----------------|-------|-----------|---------|------------------|-----------------|--------|
| 17.1 | PRV substrate → 17.5/17.7 | Existing registry/auth adapters/fixtures reused; no router trait added | `opi-ai` core as admitted | Opaque `PreparedProviderCall` + prepared-auth seam only | Reaches 17.5 via `build_harness_collection` and 17.7 via route/provenance consumption | No product cutover/alias/second registry — held | `conforming` |
| 17.2 | NXT cutover | Agent state/cancellation/queues/compaction fixtures reused | `opi-agent` core as admitted | `NextTurnState` + one validated idle replacement; no patch protocol | `Agent::prompt`/`continue_`/loop production paths | Removed symbols absent from all src | `conforming` |
| 17.3 | Evidence contract substrate | Redaction helpers/event vocabulary reused | `opi-agent` core as admitted | Typed ids/health/binding/lifecycle; only Noop+InMemory adapters | Consumed by 17.4 (health in request) and 17.7 (file adapter) | No file adapter/exporter in core — held | `conforming` |
| 17.4 | AUT cutover | Schema validation/deny hooks/built-in tools/execution routing reused | Mechanism in `opi-agent`, policy in product, as admitted | `RegisteredTool`/`ToolAuthorizer`/closed decision | Trusted assembly → loop chain at HEAD | No allow-all/hook-grant/argument mutation | `conforming` |
| 17.5 | Product provider assembly | Provider config/credential+OAuth resolvers/registry UI reused | Reference product as admitted | Constructors carry collection; no overload | One collection; zero `fn stream(` in opi-ai src | No aliases/eager auth/second registry/test-only injection | `conforming` (M3 is a post-exit remediation defect on this surface, not an admitted-surface violation) |
| 17.6 | Agent evidence runtime | Existing lifecycle emission points/retry machinery reused | `opi-agent` core as admitted | Sink/binding accepted at Agent seam; no-op default | Loop emission points at HEAD | No product cutover/core file adapter | `conforming` |
| 17.7 | Product evidence cutover | Capture option/paths/runners/diagnostics/artifact-audit patterns reused | Reference product as admitted | Existing capture inputs → completeness mapping; legacy core exports removed | Fail-closed setup in all public entries; strict manifest | No exporter/db/ActiveSnapshot fabrication/dual path | `conforming` |
| 17.8 | Legacy session migration | JSONL repository/branch+fork logic/parser/fixtures reused | Reference product as admitted | Typed route remediation on existing resume/fork returns | Lookup-only normalization; byte-preserving fixtures green | No rewrite/reader/guessing/shim | `conforming` (M4 is a post-exit defect in this path) |
| 17.9 | Assurance/docs/CI | Existing subprocess mode harnesses/CI matrix/smoke/docs reused | Assurance as admitted | None added | CI job + bilingual docs + CHANGELOG verified | No runtime source repair | `drifted` — AGENTS.md/CLAUDE.md four-flavor lockstep contract violated (MIN-11); CI evidence currency gaps (M1/M2) |

---

## 10. Residuals and Recommendations

### Priority recommendations

1. **Fix M3 + M4 together** (model-change write path): make
   `try_configure_model` validate dispatchability (or reorder
   apply-then-persist), replace the `apply_recorded_model` `.expect` with the
   typed incompatible-model diagnostic branch, and add the two regression
   tests (set-model to a lookup-only provider; resume of a session recording
   one). These are the only behavioral Majors and both live in unpushed
   commits, so they can be fixed before anything is pushed.
2. **Close the evidence-currency gap (M1/M2)**: push the 5 commits, run the
   three-platform `phase17_acceptance` matrix and the six source-ordered
   workspace gates at the final SHA, and record the run URLs. Until then,
   treat the ledger's "ran green" rows as historical (this audit's local
   HEAD results — 621 focused tests, fmt, clippy-lib, doc-check, all green on
   Windows — are the only validation attaching to `a680c5d`).
3. **Record a dated citation addendum** for the phase-17 snapshot mapping
   stale criteria_trace citations (MIN-10, INFO-24, INFO-28) and the two
   corrected residuals (PRV-005 narrowed, cross-mode asymmetries closed) to
   their HEAD locations, without rewriting the archived ledger.
4. **Route the post-exit spec tightenings to owners**: INV-007 entry
   classification + unsupported-version fixture (MIN-13/14), PRIN-004/INV-005
   permutation-matrix tests (MIN-15), and the project-trust conflict-rule
   reconciliation (MIN-16) are future-work clauses at phase-owned boundaries;
   they need a tasked phase or remediation cycle, not silent acceptance.
5. **Hygiene cluster** (low risk, mechanical): phase-history comments (MIN-01),
   CLAUDE.md flavor contract (MIN-11), AUT-003 phrasing (MIN-08), the four
   test-strength gaps (MIN-19/20/21, INFO-12), and the duplicated-code
   clusters (MIN-02..05).

### Residual risk statement

No Blocker was found. The 4 Majors split evenly between evidence currency
(M1/M2 — resolved by pushing and running CI) and post-exit behavioral
regressions confined to the model-change/resume path for lookup-only
extension providers (M3/M4 — fixed by the one-path correction above). The
audit's own execution re-validated the focused acceptance surface at HEAD
with zero failures, and the doc/standards contracts (doc-check, fmt,
clippy-lib) pass at HEAD.
