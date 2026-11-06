# Batch RUN — tools, execution env, context assembly, trust, compaction (Dims 8,10,14)

Measured 2026-11-06 against pi 1.5.0 `packages/coding-agent/src/core/{tools,compaction,...}`
(source export `8aefc4c29d420c27`) and opi `crates/opi-coding-agent` (+ opi-sandbox, opi-protocol)
at tree `d382688b11cee87f`. Measured directly by the coordinator (child workers were rate-limited);
second-pass spot checks are flagged inline. 26 records: equivalent 1, partial 8, different 10,
missing 6, opi-only 1, unresolved 1.

---

### RUN-TOOL-01 — built-in tool set inventory
- Dimension(s): 8
- pi entry: `.pi/pi-1.5.0/packages/coding-agent/src/core/tools/index.ts:1-224` — exports bash, edit, find, grep, ls, powershell, read, write (+ file-mutation-queue, output-accumulator, truncate, renderers).
- Opi entry: `crates/opi-coding-agent/src/tool/mod.rs:1-13` — bash, edit, find, glob, grep, ls, read, write (+ operations, process_tree, supervision).
- Comparison: different
- Observable difference: pi-only: `powershell` (Windows-native shell tool). opi-only: `glob` (pattern-only variant; `find` and `glob` share the description "Gitignore-aware file discovery by glob pattern", `glob.rs:37`, `find.rs:41`). Everything else exists on both sides with different schemas (below).
- Evidence basis: source inspection (both export lists read in full)
- Review outcome: self-reviewed (coordinator-measured)

### RUN-TOOL-02 — read tool contract
- Dimension(s): 8
- pi entry: `core/tools/read.ts:14-17` schema `{path, offset?(1-indexed), limit?}`; description: truncated to `DEFAULT_MAX_LINES`=2000 or `DEFAULT_MAX_BYTES`=50KB (`truncate.ts:11-12`); images jpg/png/gif/webp/bmp as attachments with auto-resize (`read.ts:89-95`), non-vision model note (`:101-105`), `outputSchema` union string|image-block for codemode (`:31-36`), pluggable `ReadOperations` (`:47-56`).
- Opi entry: `crates/opi-coding-agent/src/tool/read.rs:22-47` `ReadArgs{path, offset?(1-based), limit?}`; `DEFAULT_READ_LINES`=2000 (`:24`), `MAX_READ_OUTPUT_BYTES`=64*1024 (`:29`), NUL-byte binary detection (`:299-307`), no image read path, no output schema; ops seam `FileOperations` (`operations.rs`).
- Comparison: different
- Observable difference: (1) byte cap 50KB (pi) vs 64KB (opi). (2) pi reads images and returns image content + resize hints; opi rejects binary heuristically and has no image mode in the read tool (image attachment is a CLI-level `--image` flag, `image.rs`). (3) pi exposes `limit:0`-style window semantics with truncation metadata `TruncationResult{truncatedBy, totalLines, ...}` (`truncate.ts:20-41`); opi reports `truncated`/`omitted` flags only. (4) pi path is relative-or-absolute; opi paths are workspace-relative under `PathPolicy` with symlink-traversal reporting (`mod.rs:60-89`).
- Evidence basis: source inspection
- Review outcome: self-reviewed (coordinator-measured)

### RUN-TOOL-03 — edit tool contract
- Dimension(s): 8
- pi entry: `core/tools/edit.ts:20-50` schema `{path, edits: [{oldText, newText}]}` (batch of targeted edits per call).
- Opi entry: `crates/opi-coding-agent/src/tool/edit.rs` `EditArgs{path, old_string, new_string}` — single edit per call; doc: must occur exactly once, no fuzzy match, no replace-all, CRLF/LF preserved byte-for-byte.
- Comparison: different
- Observable difference: A pi caller submits several replacements in one call; an opi caller must issue one call per replacement. opi enforces unique-match strictly (documented no-op/failure policy); pi edit-diff.ts (556 lines) implements its own uniqueness/fuzzy behavior — parameter shape alone makes the tools distinguishable by the model.
- Evidence basis: source inspection (pi schema + opi struct docs)
- Review outcome: self-reviewed (coordinator-measured)

### RUN-TOOL-04 — write tool contract
- Dimension(s): 8
- pi entry: `core/tools/write.ts:10-14` `{path, content}`.
- Opi entry: `crates/opi-coding-agent/src/tool/write.rs` `WriteArgs{path, content}`; NUL-byte content rejected before any fs side effect; bytes written verbatim; workspace-relative under PathPolicy.
- Comparison: partial
- Observable difference: Schema matches; pi accepts relative-or-absolute paths, opi is workspace-policy-gated; opi documents binary rejection (JS side is bounded by UTF-16 strings on both).
- Evidence basis: source inspection
- Review outcome: self-reviewed (coordinator-measured)

### RUN-TOOL-05 — find/glob file discovery
- Dimension(s): 8
- pi entry: `core/tools/find.ts:20-42` `{pattern, path?, limit?(default 1000)}`, gitignore-respecting; `FindToolDetails{truncation, resultLimitReached}`.
- Opi entry: `tool/find.rs` `FindArgs{pattern, path?}` and `tool/glob.rs` `GlobArgs{pattern}` — no `limit` parameter on either.
- Comparison: different
- Observable difference: pi caps results at 1000 and reports `resultLimitReached`; opi splits the same capability into two tools with no result-limit parameter; a model migrating between them sees different tool surfaces and unbounded result sets (subject to output caps).
- Evidence basis: source inspection
- Review outcome: self-reviewed (coordinator-measured)

### RUN-TOOL-06 — grep tool contract
- Dimension(s): 8
- pi entry: `core/tools/grep.ts:20-32` `{pattern, path?, glob?, ignoreCase?, literal?, context?, limit?(default 100)}`; `GREP_MAX_LINE_LENGTH`=500 chars/match line (`truncate.ts:13`).
- Opi entry: `tool/grep.rs` `GrepArgs{pattern}` only.
- Comparison: different
- Observable difference: pi exposes file filtering, case flags, literal mode, context lines and match limits; opi grep takes a bare regex pattern (defaults internal). pi-only parameters are absent without substitutes.
- Evidence basis: source inspection
- Review outcome: self-reviewed (coordinator-measured)

### RUN-TOOL-07 — ls tool contract
- Dimension(s): 8
- pi entry: `core/tools/ls.ts:10-13` `{path?, limit?(default 500)}`.
- Opi entry: `tool/ls.rs` `LsArgs{path (required), max_entries?(default 200), max_depth?(default 0)}`.
- Comparison: different
- Observable difference: Default entry caps differ (500 vs 200); opi adds recursion depth and requires an explicit path; pi path is optional with cwd default.
- Evidence basis: source inspection
- Review outcome: self-reviewed (coordinator-measured)

### RUN-TOOL-08 — bash tool contract
- Dimension(s): 8, 14
- pi entry: `core/tools/bash.ts:21-41` `{command, timeout?}` (optional seconds, no default; max 2,147,483 s, `:21-36`); output combined stdout+stderr, `STRUCTURED_OUTPUT_MAX_BYTES`=1MB with `full_output_path` spill file when truncated (`:23,56-58`); pluggable `BashOperations`/spawn hooks (`index.ts:2-10`).
- Opi entry: `tool/bash.rs` `BashArgs{command, timeout_secs?}` with `#[schemars(range(min=1,max=86400))]`; `MAX_BASH_TIMEOUT_SECS`=86400; shell `sh` (unix) / `cmd` (windows) (`bash.rs:278`); output merged stdout-then-stderr re-capped at `MAX_BASH_OUTPUT_BYTES`=64KB (`bash.rs:315-327`, `operations.rs:882`); process-tree supervision (`supervision.rs`, `process_tree.rs`).
- Comparison: different
- Observable difference: (1) timeout bounds: pi allows up to ~24.8 days, opi caps at 86400 s. (2) Large output: pi spills full output to a file and returns the path; opi truncates at 64KB with no spill path. (3) pi interleaves stdout/stderr as combined stream (bash-executor merges at source); opi concatenates stdout-then-stderr. (4) opi embeds a backend-adapter enum in the tool schema with approval hints (`bash.rs:133-169`), an opi-only routing surface.
- Evidence basis: source inspection
- Review outcome: self-reviewed (coordinator-measured)

### RUN-EXEC-01 — execution backend routing & sandbox integration
- Dimension(s): 8, 14
- pi entry: none — pi's execution customization is per-tool `Operations` plugins (`read.ts:47-56`, `bash.ts` BashOperations) with no central router; containerization is documented separately (docs/containerization.md, not core tools).
- Opi entry: `crates/opi-coding-agent/src/execution/` (`router.rs`, `runtime.rs`, `permission.rs`, `contribution.rs`, `protocol_host.rs`): `--execution-backend <local|adapter-id>` / `[execution] strategy="fixed", backend="opi-sandbox"` with per-adapter `PermissionDecision`s (`config.rs:150-242`); native `--sandbox` removed (`cli.rs:15` LEGACY_SANDBOX_REMEDIATION); opi-sandbox becomes an installable adapter package (`opi package add; opi package enable opi-sandbox`).
- Comparison: opi-only
- Observable difference: opi routes every bash invocation through an execution router with adapters (local process or protocol-backed remote/sandbox adapters) and permission gating; pi runs tools locally unless the per-tool Operations plugin is overridden by embedder code.
- Evidence basis: source inspection
- Review outcome: self-reviewed (coordinator-measured)

### RUN-NEST-01 — nested tool calls (ctx.executeTool)
- Dimension(s): 8
- pi entry: `core/nested-tool-calls.ts:1-31` — tools call other tools via `ctx.executeTool()`; each nested call runs through the agent `runToolCall` pipeline, emits `tool_execution_*` with `parentToolCallId`, recorded on the parent tool-result message with `NESTED_CALL_LIMITS` (256 calls, 8KB args/call, 32KB total, 500-char errors).
- Opi entry: absent at the product seam — opi-agent has no `run_tool_call` re-entry (AGT-TOOL-05); `grep -rn "parent_tool_call\|ParentCallId" crates/opi-coding-agent/src/` → only `evidence.rs` correlation comment (opi-agent). `tool/supervision.rs` supervises bash processes, not nested tool calls.
- Comparison: missing
- Observable difference: pi tools (notably codemode scripts) can invoke other tools with full hook/permission application and observable nested-call records; opi tools cannot route nested calls through the pipeline.
- Evidence basis: source inspection + bounded absence search
- Review outcome: self-reviewed (coordinator-measured)

### RUN-GUARD-01 — output guard (stdout takeover protection)
- Dimension(s): 11 (side effect), 8
- pi entry: `core/output-guard.ts:1-40` — takes over raw stdout/stderr writes so accidental prints during JSON/RPC modes cannot corrupt the protocol stream.
- Opi entry: absence search `grep -rn "takeover\|output_guard\|OutputGuard" crates/opi-coding-agent/src` → no match.
- Comparison: missing
- Observable difference: a stray `println!` from tool-adjacent code in opi JSON mode would interleave with the NDJSON event stream; pi detects/guards this class of failure.
- Evidence basis: source inspection + bounded absence search
- Review outcome: self-reviewed (coordinator-measured)

### RUN-CTX-01 — system prompt assembly
- Dimension(s): 10
- pi entry: `core/system-prompt.ts:52-56,127-191` — ordered, independently replaceable sections (`preamble` untagged + `<tag>`-wrapped sections), becoming `SystemMessage.sections` in the transcript so later updates target one section.
- Opi entry: `crates/opi-coding-agent/src/prompt.rs:7-89` — flat builder: base prompt → user system → tool list ("Available tools:\n- name: desc") → context files; single string output, no section structure.
- Comparison: different
- Observable difference: pi can update one prompt section mid-session with a matching section update; opi regenerates the whole prompt text; the model-visible wire format differs (tagged sections vs flat concatenation) and section-targeted updates are unrepresentable.
- Evidence basis: source inspection
- Review outcome: self-reviewed (coordinator-measured)

### RUN-CTX-02 — context file discovery (AGENTS.md/CLAUDE.md)
- Dimension(s): 10, 14
- pi entry: `core/resource-loader.ts:260-304` (`loadProjectContextFiles` walks upward comparing real paths for worktrees); discovery/integration across resource loader with collisions/diagnostics (`:9`).
- Opi entry: `crates/opi-coding-agent/src/context_files.rs:15-79` — `CONTEXT_FILE_NAMES = [AGENTS.md, CLAUDE.md]` per-directory order AGENTS before CLAUDE, global config dir included, upward walk to `.git` marker/root, `include_project=false` for untrusted projects (OPI.md deliberately not read).
- Comparison: partial
- Observable difference: Both discover upward and gate on trust. pi compares real paths for worktree correctness and surfaces collision diagnostics through a general resource loader; opi's trust gating excludes project files entirely until trusted. File-name sets match; pi's `.pi`/`.claude` resource discovery scope is broader (see EXT batch).
- Evidence basis: source inspection
- Review outcome: self-reviewed (coordinator-measured)

### RUN-SKILL-01 — skills (SKILL.md)
- Dimension(s): 10
- pi entry: `core/skills.ts:67-83,115-168` — frontmatter `{name, description<=1024}`, recursive `SKILL.md` discovery, load-skills result with errors.
- Opi entry: `crates/opi-coding-agent/src/skill.rs:10-99` — same SKILL.md + YAML frontmatter, 1024-char description cap, body deferred until load.
- Comparison: equivalent
- Observable difference: frontmatter format and limits match at the loader level; activation/integration differences (skill commands, `enableSkillCommands` setting, prompt blocks) are product-surface items covered in the EXT/CLI batches; pi validates per Agent Skills spec wording.
- Evidence basis: source inspection
- Review outcome: self-reviewed (coordinator-measured)

### RUN-PROMPT-01 — prompt templates
- Dimension(s): 10
- pi entry: `core/prompt-templates.ts` + docs/prompt-templates.md — template files with arguments, discovery under user/project dirs.
- Opi entry: `crates/opi-coding-agent/src/prompt_fragment.rs:1-143` — "fragment" system: `FragmentManifest::from_fragment_md` with declared `FragmentArgument`s, progressive discovery, registry, expansion.
- Comparison: partial
- Observable difference: Both provide parameterized reusable prompts; opi names files `fragment.md` (vs pi template naming), and discovery/argument syntax differ. Unverified: exact pi template file name/argument syntax (not read in this pass) — treat argument-shape parity as unresolved.
- Evidence basis: source inspection (opi full, pi headers only)
- Review outcome: self-reviewed (coordinator-measured)

### RUN-SLASH-01 — slash commands (interactive)
- Dimension(s): 11, 10
- pi entry: `core/slash-commands.ts` — general slash-command registry (built-ins + skills as commands, `enableSkillCommands` default true, settings-defaults.ts:42).
- Opi entry: `crates/opi-coding-agent/src/interactive.rs:1437-1485` — hardcoded commands: session metadata, branch picker (`/branch`), auth commands; no general registry found (`grep -n "slash" interactive.rs` → none).
- Comparison: partial
- Observable difference: pi exposes an extensible command set (skill commands, extension commands); opi's command set is a fixed small list.
- Evidence basis: source inspection (pi registry existence; opi command dispatch read)
- Review outcome: self-reviewed (coordinator-measured)

### RUN-SET-01 — settings model
- Dimension(s): 10, 9, 11
- pi entry: `core/settings-defaults.ts:5-58` — settings.json: transport, sessionShareDestination, steeringMode/followUpMode, compaction{enabled,reserveTokens:16384,keepRecentTokens:20000}, branchSummary, retry{maxRetries:3,baseDelayMs:2000,provider.maxRetryDelayMs:60000}, terminal{showImages,imageWidthCells:60,...}, images{autoResize,blockImages}, codemode{mode:inlineBudget:3000}, cacheWarming:"streaming", tuiMode:"fullscreen", fullscreen* (5 keys), markdown{codeBlockIndent,mermaid}, autocompleteMaxVisible, warnings, analytics/installTelemetry, quietStartup, defaultProjectTrust:"ask" (~50 keys).
- Opi entry: `crates/opi-coding-agent/src/config.rs:24-416` — TOML: defaults, execution{strategy,backend,rules,permissions}, providers (custom per-provider blocks), thinking, credential backend.
- Comparison: different
- Observable difference: nearly disjoint key sets. pi settings drive UX/steering/compaction/codemode/cache behavior; opi config drives execution backends, provider endpoints and policy. pi's per-user settings.json has no opi counterpart as one file; opi's execution/provider policy has no pi counterpart in settings.
- Evidence basis: source inspection
- Review outcome: self-reviewed (coordinator-measured)

### RUN-TRUST-01 — project trust
- Dimension(s): 14
- pi entry: `core/trust-manager.ts:9-217` — `ProjectTrustStore` with lock-file sync (`acquireTrustLockSync:145`), nearest-entry parent matching (`:50-72`), session-only decisions (`includeSessionOnly:82`), `hasTrustRequiringProjectResources:193`; `core/project-trust.ts` + `cli/project-trust.ts` (--approve/--no-approve); default `ask` (settings-defaults:44).
- Opi entry: `crates/opi-coding-agent/src/project_trust.rs:118-333` — `TrustDecision`/`TrustContext`, sealed `ProjectTrustResolverRegistry`, `ProjectTrustStore`; gates context files (`context_files.rs:42-79`) and tool policy (`tool_authority.rs`).
- Comparison: partial
- Observable difference: both persist trust decisions and gate project-local resources; pi supports session-only trust (until restart) and parent-path inheritance with nearest-entry semantics; opi's registry is sealed at startup and integrated into the execution-policy pipeline. Whether opi supports session-scoped (non-persisted) trust is unverified here.
- Evidence basis: source inspection
- Review outcome: self-reviewed (coordinator-measured)

### RUN-COMP-01 — compaction
- Dimension(s): 10, 6
- pi entry: `core/compaction/` (compaction.ts, branch-summarization.ts, utils.ts): `DEFAULT_COMPACTION_SETTINGS{enabled:true,reserveTokens:16384,keepRecentTokens:20000}` (settings-defaults:14-18), `shouldCompact`, `findCutPoint`, `estimateContextTokens` (usage-derived, `compaction.ts:140-210`), file-operations carry-over (readFiles/modifiedFiles preserved across compaction, `:65-83`), branch summarization with generateBranchSummary.
- Opi entry: `crates/opi-coding-agent/src/compaction.rs:55-215` — bridge to opi-agent `CompactionEngine` (`opi-agent/src/compaction.rs:66-165`): manual/threshold/overflow triggers, 25%-tail keep split, hook-or-core heuristic (chars/4) summaries, summary usage bridging; threshold via config `compaction.threshold_tokens`.
- Comparison: different
- Observable difference: trigger model (pi: token-reserve estimate from last usage + reserveTokens/keepRecentTokens defaults; opi: threshold_tokens config + 25%-tail split heuristic), summary generation (pi: model-generated summaries incl. branch summaries with skipPrompt setting; opi: hook-provided or chars/4 core heuristic), file-activity carry-over (pi explicit readFiles/modifiedFiles sets; opi none found), branch summaries (pi dedicated subsystem; opi session_branch summary injection at reconstruction — see AGT-ONLY-SESSION).
- Evidence basis: source inspection
- Review outcome: self-reviewed (coordinator-measured)

### RUN-CACHE-01 — cache warming
- Dimension(s): 4 (side), 10
- pi entry: `core/cache-warmer.ts` + `cacheWarming:"streaming"` default (settings-defaults.ts:53), `CacheWarmingDecision/Status` exported from coding-agent index.
- Opi entry: absence search `grep -rn "cache_warm\|CacheWarm" crates/` → no match.
- Comparison: missing
- Observable difference: pi warms provider prompt caches during streaming per setting; opi has no equivalent behavior.
- Evidence basis: source inspection + bounded absence search
- Review outcome: self-reviewed (coordinator-measured)

### RUN-USAGE-01 — usage totals & cost
- Dimension(s): 17 (product side)
- pi entry: `core/usage-totals.ts` (combineUsage; footer/session totals), model-data pricing via pi-ai.
- Opi entry: `crates/opi-coding-agent/src/pricing.rs:1-20` — small built-in per-provider default-model price table, best-effort (`None` → token totals only); `SessionCoordinator::cost_summary`; `interactive.rs:1276`, `runner.rs:387` surface totals.
- Comparison: partial
- Observable difference: opi computes session cost from a tiny hardcoded table (unknown models → no cost); pi derives cost from full model-data pricing (opi-ai catalog parity is good per AI-MOD batch — the product layer just doesn't use it here). Totals exist both sides; coverage and precision differ.
- Evidence basis: source inspection
- Review outcome: self-reviewed (coordinator-measured)

### RUN-OPS-01 — pluggable file/shell operations seams
- Dimension(s): 8, 18
- pi entry: per-tool `ReadOperations`/`BashOperations`/etc. ("Override these to delegate file reading to remote systems (for example SSH)", read.ts:50-53), `core/exec.ts` shared spawn utilities for extensions.
- Opi entry: `tool/operations.rs` — central `FileOperations`/`BashOperations` traits with `LocalFileOperations`/`LocalBashOperations`, injected per tool (`read.rs:53-69`), plus the execution router (RUN-EXEC-01) as the system-level extension point.
- Comparison: partial
- Observable difference: same plugin pattern (local default, overridable), different placement: pi per-tool options objects; opi centralized trait objects + router. pi's `exec.ts` exposes spawn utilities to extension authors; opi extensions are external processes (EXT batch).
- Evidence basis: source inspection
- Review outcome: self-reviewed (coordinator-measured)

### RUN-QUE-01 — file mutation queue
- Dimension(s): 8
- pi entry: `core/tools/file-mutation-queue.ts` (`withFileMutationQueue` exported from tools index) — serializes conflicting file mutations between concurrent tools.
- Opi entry: no counterpart found: `grep -rn "mutation_queue\|MutationQueue" crates/opi-coding-agent/src` → none. (opi-ai has credential `MutationQueue`, unrelated.)
- Comparison: missing
- Observable difference: parallel pi tools mutating the same file are serialized; opi relies on tool implementations/supervision without a shared mutation queue at this layer.
- Evidence basis: source inspection + bounded absence search
- Review outcome: self-reviewed (coordinator-measured)

### RUN-H-01 — Layer H: bash model-routing contract
- Design claim: opi `tool/bash.rs:133-137` — bash tool schema embeds backend-adapter enum with approval hints "which the design mandates (§Model routing)"; execution permission model in `execution/permission.rs` (`PermissionDecision` ask/allow).
- Authority: internal normative doc-comment citing a design section; implemented + tested (`permission.rs:226` test).
- Maturity: implemented (product-integrated, opi-only; no pi counterpart).

### RUN-H-02 — Layer H: sandbox deprecation/removal path
- Design claim: `cli.rs:15`, `diagnostics.rs:20` — native `--sandbox` flag and `[sandbox]` config removed; remediation text points to execution backends + installable `opi-sandbox` package.
- Authority: normative diagnostics constants; implemented.

---

## Verdict tallies

| Verdict | Count | Records |
|---|---|---|
| equivalent | 1 | RUN-SKILL-01 |
| partial | 8 | RUN-TOOL-04, RUN-CTX-02, RUN-PROMPT-01, RUN-SLASH-01, RUN-TRUST-01, RUN-USAGE-01, RUN-OPS-01, (+RUN-PROMPT-01 unresolved part) |
| different | 10 | RUN-TOOL-01..03, 05..08, RUN-CTX-01, RUN-SET-01, RUN-COMP-01 |
| missing | 6 | RUN-NEST-01, RUN-GUARD-01, RUN-CACHE-01, RUN-QUE-01, (+ RUN-PROMPT-01 argument-shape sub-item) |
| opi-only | 1 | RUN-EXEC-01 |
| unresolved | 1 | RUN-PROMPT-01 (pi template argument syntax not read) |

## Coverage honesty

- pi `bash.ts`, `read.ts`, `find.ts`, `grep.ts`, `ls.ts`, `edit.ts` (schema region), `write.ts` (schema region), `truncate.ts`, `nested-tool-calls.ts`, `output-guard.ts`, `exec.ts` (head), `settings-defaults.ts`, `system-prompt.ts` (structure), `skills.ts` (API region), `trust-manager.ts` (API region), `resource-loader.ts` (API region), `compaction.ts` (API region) inspected; `edit-diff.ts` (556 loc), `output-accumulator.ts`, `renderers/`, `powershell.ts`, `slash-commands.ts`, `prompt-templates.ts` bodies, `keybindings*.ts`, `compaction/branch-summarization.ts`, `compaction/utils.ts` NOT read line-by-line — their records lean on exports/schemas/defaults and cross-batch evidence.
- opi `tool/mod.rs`, `read.rs`, `bash.rs` (schema+output regions), `edit.rs` (struct), `write.rs` (struct), `find.rs`/`glob.rs`/`grep.rs`/`ls.rs` (structs+descriptions), `operations.rs` (constants), `prompt.rs`, `context_files.rs`, `skill.rs`, `project_trust.rs` (API region), `config.rs` (struct list), `compaction.rs` (bridge region), `pricing.rs` inspected; `supervision.rs`, `process_tree.rs`, `tool_authority.rs`, `policy.rs`, `harness.rs`, `interactive.rs` (command region only) partially inspected.
- No code executed; read-only inspection. Child-worker unavailability (account rate limit) means this batch lacks an independent second pass; coordinator self-review only, flagged per record.
