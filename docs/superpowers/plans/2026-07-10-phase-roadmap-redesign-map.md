# Wayfinder Map — opi Phase-Roadmap Redesign (pi-0.80.6 realignment)

Created 2026-07-10. This is a **wayfinder decision-map**: the planning layer that
produces opi's redesigned phase roadmap. Each ticket is one design decision sized
to a single agent session, to be resolved (with `/grilling` + `/domain-modeling`)
before its phase's design doc is written. Phase **execution** tracking stays in
`.opi-impl-state.json` via the `opi-implement` skill; this map does not duplicate
it. Tickets resolve to a design doc per phase (e.g.
`docs/superpowers/specs/YYYY-MM-DD-phase14-...-design.md`), which `opi-implement`
then breaks into implementation tasks.

The roadmap itself is committed to `docs/opi-spec.md` §15 as each phase's design
is settled (a separate step — editing `opi-spec.md` triggers phase4/phase6
ledger re-sync per project convention).

## Destination

A redesigned `docs/opi-spec.md` §15 roadmap (phases 14+) that reconciles opi
with the pi-0.80.6 realignment under **posture B (strategic gap-closing)**: a
selected set of ecosystem gaps promoted to real phases, each closed with a
Rust-native design; the rest kept as Future Ecosystem Candidates (§3.3). The map
is done when every ticket below is resolved and each phase has a reviewed design
doc — i.e. the way to the redesigned roadmap is clear and no decisions remain
open before implementation.

## Notes

- **Domain.** opi is a Rust reimplementation of pi (earendil-works/pi). Edition
  2024, Rust 1.97+. Four crates: `opi-ai` (LLM API, no internal deps), `opi-tui`
  (terminal UI, no internal deps), `opi-agent` (→ `opi-ai`; agent runtime, tools,
  sessions, compaction), `opi-coding-agent` (→ `opi-ai`, `opi-agent`, `opi-tui`;
  produces the `opi` binary).
- **Load-bearing invariant (verified-accurate, rephrased).** `opi-agent` must
  **not construct providers or own provider/auth configuration**; provider
  instances are built by `opi-coding-agent/provider_factory` and injected via
  `AgentLoopContext.provider`. `opi-agent` *does* call `provider.stream()` at
  `agent_loop.rs:118` and holds `Box<dyn Provider>` — so it is not "LLM-free" in
  the call sense; the accurate rule is about *construction and config ownership*.
  Any product behavior needing a provider call or IO lives in `opi-coding-agent`
  and injects via hook/trait seams (`CompactionHooks`, `AgentHooks`,
  `Extension`). The older phrasing "opi-agent stays LLM-free" is imprecise; do
  not use it to evaluate tickets.
- **Skills every ticket session should consult.** `/grilling` + `/domain-modeling`
  for resolution; the `opi-realign` audit (`docs/realign/2026-07-10-opi-vs-pi-0.80.6.md`,
  152 deltas / 18 dims) for the gap inventory; `opi-spec.md` §2.1 Non-Goals, §2.2
  pi Design Boundaries, §3.2 Rust-Native Redesigns, §3.4 Alignment Vocabulary for
  philosophy.
- **Standing preferences.** Rust-native over parity-copy (§3.2); avoid `unsafe`,
  prefer safe abstractions (CLAUDE.md); new crates enter via
  `[workspace.dependencies]`, never directly in a crate's `Cargo.toml`; this host
  uses `python` not `python3`; editing `opi-spec.md` triggers phase4 + phase6
  ledger snapshot re-sync.

## Decisions so far

- **[Posture B — strategic gap-closing]** — promote selected ecosystem gaps to
  real phases with Rust-native designs; keep the rest as Future Ecosystem
  Candidates. *(resolved 2026-07-10)*
- **[Promote four clusters: A / K / D+E / C+F]** — Provider/Auth, Safety/Sandbox,
  Agent Intelligence, Extension+TUI all become phases. Catalog breadth (B),
  packaging (G), export/share (H), image-generation (I), orchestrator (J) stay
  Future Ecosystem. *(resolved 2026-07-10)*
- **[Sequence 14 → 15 → 16 → 17]** — Provider/Auth → Safety/Sandbox → Agent
  Intelligence → Extension+TUI. Value/risk-driven; verified to have **no hidden
  hard inter-phase dependency**. *(resolved 2026-07-10)*
- **[Existing Phase 14 (TUI polish) is complementary, not absorbed]** — the
  `2026-06-24-phase14-tui-product-polish-design.md` doc explicitly carves
  engine/overlay/editor OUT as non-goals (lines 49, 66); it is pure built-in
  polish (view-models, pickers, transcript, a11y) that runs on top of whatever
  engine Phase 17 ships, sequenced after the Phase 17 engine foundation.
  *(resolved 2026-07-10, corrected by verification)*
- **[Map home: `docs/superpowers/plans/`]** — local markdown, network-free,
  consistent with project convention; tickets are sections in this file.
  *(resolved 2026-07-10)*
- **[Adversarial verification run `wf_1c91c099-145`]** — 6-verifier check (phases
  14-17 + Rust-idiom survey + dependency/philosophy). 27 errors, 25 missing
  items, 22 ticket refinements folded into the tickets below. Headline
  corrections: crate-name fixes (`keyring-core`, `landlock`, +new `fs4`/`win32job`/
  `extrasafe`/`secrecy`/`ignore`/`ratatui-image`); T9 premise corrected (ratatui
  already cell-diffs); T10 split into three separable mechanisms; invariant
  rephrased; spec-drift (branch-summary Phase 14→16) flagged; two promotable
  gaps surfaced (read-tool image, TTY inference). *(resolved 2026-07-10)*
- **[T1 Credential store contract resolved]** — keychain (`keyring-core`) primary + env-var fallback, **no plaintext file**; `CredentialStore` trait + `Credential` type in opi-ai, impls + `CredentialResolver` in opi-coding-agent; additive `AuthDescriptor::StoreCredential` + 3-state `CredentialSource` probe; `fs4` single global lock (acquire-then-re-read); `secrecy` at store boundary (not end-to-end). `/login`//`/logout` UX + OAuth flow deferred to T2. *(resolved 2026-07-11, grilling D1–D5; detail in ticket T1)*
- **[T2 OAuth + per-request auth resolved]** — provider holds injected `AuthSource` enum (Baked/Store/EnvOAuthToken), resolves per `stream()` (trait + live path unchanged, collection off live path); 3 OAuth providers (Anthropic/Copilot/Codex) full parity; `OAuthProvider` trait + `OAuthCredential` in opi-ai, impls + registry in opi-coding-agent; double-checked-locking refresh (5-min skew, lock-during-HTTP); revocation = non-retryable auth-invalid + `CredentialRevoked`, no auto-relogin mid-stream; `LoginPresenter` seam with manual-paste fallback; `ANTHROPIC_OAUTH_TOKEN` recognized; per-call override OUT (fog). Clears opi-spec:1641 entry condition. *(resolved 2026-07-11, grilling D1–D8; detail in ticket T2)*
- **[T3 opi-ai Request enrichment resolved]** — scope 3a/3b/3c/3e in, 3d deferred (fog); `Request` += timeout/extra_headers/cache_retention/session_id (no retry budget — that's opi-agent); `Usage`/`CostBreakdown` += cacheWrite1h/reasoning; `Provider` += async `refresh_models` default None; new cross-provider `ModelCapabilities` on `ModelInfo` drives Anthropic `cache_control` markers (system + last text + last tool, pi-pattern ttl). *(resolved 2026-07-11, grilling D1–D3; detail in ticket T3)*
- **[T4 OS-native sandbox resolved]** — subprocess-tree-only (opi unconfined), opt-in defense-in-depth **not** a security boundary; L0 (process-group/Job-Object tree-kill) default-on, L1/L2/L3 opt-in `strict` (default off); Linux via `pre_exec`+`extrasafe`+`landlock` (parent-build/child-apply, no opi `unsafe` block), Windows L0-only `win32job` (assign-after-spawn, race documented), macOS L0+`sandbox-exec` deny-overlay; fail-open-with-diagnostic + opt-in `require`; bash is the sole `strict` surface (adapters L0 only — strict deferred to per-adapter declarations; in-process tools + `credential_process` out). Verified: opi has **no subagent/Task tool**. *(resolved 2026-07-11, grilling D1–D8 + spawn-audit `wf_d3373e28-329`; detail in ticket T4)*
- **[T5 per-tool Operations seam resolved]** — `FileOperations` (read+write+edit) + `BashOperations` (bash) traits in **opi-coding-agent** (pi precedent: `pi-agent-core` has no tools/Operations, all 7 live in `pi-coding-agent`; corrects the ticket's "like PathPolicy" premise — PathPolicy is also in opi-coding-agent); pure FS/exec backend layering **below** PathPolicy pre-flight; confinement authoritative for local, remote backend owns its own; constructor `Arc<dyn>` injection, local default, no remote backends in Phase 15; T4 sandbox relocated **inside local `BashOperations::exec`**, FileOperations unsandboxed (PathPolicy-guarded); nav tools stay local. *(resolved 2026-07-11, grilling D1–D6; detail in ticket T5)*
- **[T6 project-trust gate resolved]** — gates LOADING of project-local resources incl. **project-local adapter declarations** (untrusted project's adapters don't start — closes the native-child-process blast-radius gap T4 flagged, via declaration-load gating); user-global + tool execution ungated; project `AGENTS.md`/`CLAUDE.md` gated (**deviates from pi** — prompt-injection channel); pi-style `trust.json` store (flat `Map<path,bool>`, ancestor walk, realpath, `fs4` lock) in opi-coding-agent; full ask UX (`AppState::AwaitingTrust`, non-interactive defaults untrusted); `--trust`/`--no-trust` + `[defaults] default_project_trust` (global-only) + `/trust`; `project_trust` extension hook (user-global/CLI -e only, minimal pre-trust UI). **Phase 15 fully resolved (T4+T5+T6).** *(resolved 2026-07-11, grilling D1–D6; detail in ticket T6)*
- **[T7 Skills/templates runtime resolved]** — skills + fragments stay distinct mechanisms; `/skill:name` injects raw `<skill>` body + appended args (NO substitution — corrects ticket's bash-style-for-skills misread; pi `agent-session.ts:1274` appends); fragments keep opi's `{{name}}` declared-arg engine (wire dead code); skill discovery recursive+`.gitignore` via existing `ignore="0.4"` dep (NOT new — corrects T7d), fragments single-level, no symlinks; `SystemPromptBuilder` emits pi-style `<available_skills>` XML replacing the inert prose list (read-tool-gated, finally wires the dead `disable_model_invocation`); fragments dropped from system prompt; `/skill:`+`/fragment:` text-expansion at interactive + non-interactive + steer + follow-up (expansion at `harness.steer/follow_up` + the rpc.rs `self.control.*` branch; RPC text-only, dedicated `SdkCommand` → T11); mode-sensitive failure handling (interactive errors on unknown — deliberate pi divergence); **opi-agent stays skill-free** (verified `wf_12096538-a73`, 5 verifiers); depends on T6 (Phase 15) filtering skills/fragments at discovery. Phase 16 design doc deferred until T8+T9 resolve. *(resolved 2026-07-11, grilling D1–D7; detail in ticket T7)*
- **[T8 LLM compaction & branch-summary resolved]** — scope (a)+(b)+(d) IN / (c)+(e) DEFER; async `generate_summary` boxed-future `Result<Option<String>,AgentError>` + `CancellationToken` (§9.5 mandates `Result` for visible overflow error; dyn-compat mandates boxed future over `&dyn CompactionHooks`); `SessionCoordinator` holds `Arc<dyn CompactionHooks>` via `.with_hooks()` + first provider-backed hook impl (`ProviderCompactionHooks`) in opi-coding-agent; new `Agent::provider_arc()` accessor (minimal cascade, `Agent::new` unchanged); `keep_recent_tokens=20000` added to `CompactionConfig` (separate from trigger `threshold_tokens`), walk-back + User-turn snap, **no split-mid-turn** (deliberate pi divergence); `move_to` on `SessionCoordinator` + `CodingHarness::generate_branch_summary` + `fork_session::ForkTarget`; no-op idempotent re-move; `/move <entry_id>` + RPC `Move` (picker → Phase 17); §15 retarget (`:1595-98`→16, `/export`→17) batched with design doc. 10 ticket-premise corrections surfaced (T8d not a pure refactor; two coordinators; no provider_arc today; etc.). Phase 16 design doc still BLOCKED on T9. *(resolved 2026-07-11, grilling Q1–Q8; deep pass `wf_ed9ff140-12c`; detail in ticket T8)*
- **[T9 Read-tool inline image resolved]** — inline via a companion `OutputContent::Text` + `OutputContent::Image` (Q5′: the wire layer can't build a rich placeholder since `OutputContent::Image` carries only source+media_type, so the tool emits a label Text and degraded Image-arms emit ""); wire fix IN scope — 3 fix-sites/4 providers (Anthropic content-block array, Gemini/Vertex `inline_data`, Bedrock Converse; text-only tool_result stays byte-identical, array only when Image present) + 2 degrade-sites/5 OpenAI-compat; decode+resize via the `image` crate (1568px/5MiB, preserve media_type, GIF→first-frame; `with_guessed_format` magic-byte detection folds MIME into decode); early-branch in `execute` over T5 `FileOperations`; always-on, no knob; fire-and-forget; `ReadFileError::ImageDecode` + opi-agent `FsToolError` variant. Phase 16 design doc unblocks. *(resolved 2026-07-11, grilling Q1–Q8 + Q5′ + verification `wf_a1462c1a-da9`; detail in ticket T9)*
- **[T10 TUI engine foundation resolved]** — event-driven render loop (`tokio::select!` over crossterm `EventStream` + `watch<u64>` wake from the subscribe callback + pending `JoinHandle` + **streaming-redraw throttle** armed only while streaming) + dirty-gate (load-bearing during streaming, defensive at idle) + `Event::Resize`-drop fix; full-pi-parity `OverlayStack` in opi-tui (9 anchors, `SizeValue`, `OverlayHandle`, `focusOrder` z-order, blocked-focus-restore) via ratatui Clear+Rect (NOT pi's line-splice; opaque-only, capability-equivalent) + explicit `FocusTarget` model + generational `OverlayHandle` + 3-picker migration; ratatui cell-diff premise confirmed; `event-stream` feature enabled (futures-core only); Viewport::Inline + animation tick + palette rejected; tui-popup not adopted; **full-parity is T13-substrate, not picker-driven** (1/9 anchors exercised today); opi-agent invariant preserved (zero TUI/crossterm refs); **T13 partially unblocked** (rendering substrate only — still owns `ExtensionUIContext` trait); T16/T17/T18 graduated for editor+keybindings / terminal-image / capability-negotiation. *(resolved 2026-07-11, grilling Q1–Q5; research `wf_95882464-4dc`; verification `wf_ffa4a66f-6e9`; detail in ticket T10)*
- **[T11 Extension registry v2 policy resolved]** — (a) dynamic registration DEFERRED (keep `Arc<Vec>` + startup lock; no driver, reads dominate, `CompositeHooks` snapshots); (b) inter-extension EventBus DEFERRED (one-way host→ext flow, zero consumers; pinned 5-point fallback — new `ExtensionEvent` type, opi-agent trait seam, STATIC subscription, `Value` payload, fire-and-forget broadcast; re-trigger on T13 need; deliberate pi divergence); (c) RPC breadth = principled tiered growth NOT blanket parity (corrected ~35→verified 31; first-class `SdkCommand` only for mid-turn/streaming/negotiation, else `extension_command` which is `ERR_AGENT_BUSY`-blocked mid-turn; commands ride with implementing tickets T1/T2/T3/T6/T8; EXCLUDE `set_tools` + top-level bash + raw hatch; `SDK_SCHEMA_VERSION` stays 3). Phase 17 design doc deferred until T12+T13+T14 resolve. *(resolved 2026-07-11, grilling Q1–Q3; verification `wf_5e5e7cac-509`; detail in ticket T11)*
- **[T12 Extension UI: inline renderers resolved → DEFERRED]** — ticket framing corrected: NOT one separable hook but 4 coupled pieces (`MessageRenderer` trait+primitive in opi-tui, `Message.custom` field, `AgentEvent::CustomMessage` + `interactive.rs` streaming conversion, `MessageList`/`Shell` dispatch) for a ZERO-producer registry (`AgentMessage::Custom` exists `message.rs:20` but only tests construct it; harness drops it `_=>{}`). Entry-renderer half DROPPED (no `SessionEntry::Custom`). Architecture pinned + crate-correct: `MessageRenderer`/`EntryRenderer` traits in opi-tui, registered via opi-coding-agent, opi-agent `Extension` untouched. Pinned ratatui-free primitive: `RenderedContent{Lines(Vec<RenderedLine>),Empty}` + `RenderColor` via existing `parse_color`/`THEME_TOKENS`; REJECT `Box<dyn Widget>` + `Vec<ratatui Line>`; flat lines only; width hint at call-time (pi divergence); dispatch key = `Custom.kind` (snake_case, not `customType`). Re-trigger: a concrete extension needs structured transcript rendering beyond a formatted system line. *(resolved 2026-07-11, grilling Q1–Q2; verification `wf_9dd7bf3e-8a6`; detail in ticket T12)*
- **[T13 Extension UI: overlay/dialog resolved → DEFERRED]** — zero in-tree drivers (JSONL adapter fire-and-forget; T6 AwaitingTrust + T2 LoginPresenter design-only — `AppState` `lib.rs:138` has no AwaitingTrust, grep finds no LoginPresenter/ExtensionUiContext). Trait location VERIFIED = opi-coding-agent (agent_loop calls `tool.execute` never UI; `Extension` has zero UI methods). Pinned concurrency: inject `Arc<dyn ExtensionUiContext>` at `build_tools` (NOT a `Tool::execute` widening — 4 args, breaks every tool); TWO transports — interactive via `PendingUiRequest`+oneshot into `Arc<Mutex<TuiState>>` (no schema change; `subscribe` is SYNC so oneshot pre-stored) + RPC via `AgentEvent::UiRequest` (additive, schema stays 3) + `SdkCommand::ui_response` (first-class; `extension_command` `ERR_AGENT_BUSY`-blocked) + pending-registry `HashMap<RequestId,oneshot>`. Deadlock guard achievable (render reads `TuiState` lock-free, bypasses `harness.lock` at `interactive.rs:655`). FIFO serialize (pi single-overlay); ESC→default+fail-tool-only; non-interactive default = `ToolError::ExecutionFailed` (the real primitive, not aspirational fail-the-turn). Pinned split-trait: `BlockingDialogUi`(select/confirm/input/notify on `PickerOverlay`, no OverlayStack dep — pi's own `ProjectTrustContext.ui` is this slice) + `PersistentOverlayUi`(setWidget/custom behind T10 OverlayStack; `custom` uses T12 `RenderedContent`); editor→T16; opaque `ExtensionOverlayHandle`; `setWidget` string-key. Re-trigger: T6/T2 lands pause-and-ask in code OR a concrete extension needs an overlay. *(resolved 2026-07-11, grilling Q1–Q3; verification `wf_22070225-2fe`; detail in ticket T13)*
- **[T14 Provider request/response hooks resolved → DEFERRED]** — opi-spec.md:1491 already defers this surface (prereqs now design-met, consumer pending); zero in-tree consumers; drivers covered (auth→T2 AuthSource, headers→T3a extra_headers [design-only, blocked], observability→MessageEnd+Usage+trace envelope, redaction→Phase 7). Hook count CORRECTED 3→1: pi split before_provider_headers due to its replace-by-return constraint opi doesn't have (opi mutates `&mut Request` in place at `agent_loop.rs:90-101`); after_provider_response dropped (MessageEnd + on_event cover; no aggregated Response struct). Pinned: ONE hook `before_provider_request(&mut opi_ai::Request)->boxed Future<Result<(),AgentError>>` on AgentHooks (additive, non-breaking), fire BEFORE validate_request_capabilities (:102), chain base-then-extensions, seam-not-provider-call (invariant preserved; opi-agent already imports Request). Footguns: request.cancel frozen (same token as tokio::select race :128-140); abort Deny-vs-throw at trigger time. T3d streaming stays separate fog. :1491 spec amendment batched with §15 rewrite. Re-trigger: concrete consumer needs per-turn model/message/header mutation beyond T2/T3a/AgentEvent. *(resolved 2026-07-11, grilling Q1–Q2; verification `wf_9f09c0ec-3ec`; detail in ticket T14)*
- **[T15 stdin/stdout TTY auto print-mode — ACCEPTED]** — first non-deferred ticket (T11-T14 deferred no-driver; T15 has a concrete driver: `echo prompt | opi` hangs today). ACCEPT: auto-switch interactive→print when stdin non-TTY AND not `--rpc` AND no positional prompt, read piped stdin as prompt, route non-interactive (minimal addition to the `main.rs:138-181` 3-way split). stdin-only (stdout-TTY out of scope, separate bug); `--rpc` safe (checked first); positional-wins; empty stdin→existing "no prompt provided" error; `std::io::IsTerminal` (1.70+, no new dep). Standalone CLI fix, no phase, no design doc — implement directly. *(resolved 2026-07-11, grilling; grounded in `main.rs:138-181`; detail in ticket T15)*
- **[T16 Emacs line editor + KeyAction — SPLIT: Tier 1 ACCEPTED / Tier 2 DEFERRED]** — crux VERIFIED: InputEditor is render-only (`editor.rs:14-64`); the ENTIRE editing surface is append-at-end + pop-last-char (`interactive.rs:677-685`), no cursor/mid-line edit — real core gap (fix typo at pos 3 of 100-char prompt = backspace 97). Tier 1 SHIP: `cursor: usize` on TuiState + cursor-aware ops + Left/Right/Home/End/Delete raw KeyCode arms + extend 3-field Keybindings struct (NOT KeyAction enum) + reversed-cell visible cursor + fix multi-line `editor.rs:60` (Line→Text) + promote `unicode-segmentation` to workspace dep (grapheme-correct). Tier 2 DEFER: full Emacs (kill-ring/yank/undo/word-nav) + ~29-action KeyAction enum w/ conflict-detect (no driver; opi surface ~8 keystrokes, conflict-detect pays at ~12-15+). Re-trigger Tier 2: power-user request OR binding count ~12-15 OR accessibility. Decoupled from T10 (raw KeyCode dispatch). Verification `wf_3719b0e6-667` (5 agents). *(resolved 2026-07-11, grilling Q1; detail in ticket T16)*

## Map status (2026-07-11): PAUSED at the Phase 17 extension-surface boundary

T11–T16 resolved. The **extension-surface cluster** (T11(a/b/c), T12, T13, T14)
is deferred-with-pinned-designs — every one re-triggered by Phase 14/15/16 landing
in code (their substrates — T2 `AuthSource`, T3 `Request` enrichment, T6
`AwaitingTrust`, T8 `move_to` — are design-resolved only, not implemented). The
**TUI-quality cluster** (T10, T15, T16 Tier 1) ships — real default-mode bugs,
independent of 14–16.

**Decision: pause Phase 17 extension-surface charting.** The deferral pattern is
the map signalling those tickets were charted before their substrate existed;
continuing risks re-litigating seams once 14–16 land. T17/T18 (terminal-image
overhaul, capability negotiation) remain OPEN but are TUI-quality fixes likely to
ship on standalone drivers — pickable up incrementally, not blocked on this map.

**Next-value work (outside wayfinder):** (1) write the Phase 14/15/16 design docs
(T1–T9 all resolved; NO design doc exists for any phase yet — the per-phase
deliverable per the Destination) via `opi-document`; (2) implement via
`opi-implement`; (3) resume this map when 14–16 land in code → re-plan the Phase 17
extension surface against real consumers.

## Tickets

Status legend: **open** (unresolved frontier) · **blocked** · **resolved** (answer
recorded, linked from Decisions so far). Types: `grilling` · `research` ·
`prototype` · `task` (see wayfinder ticket types).

### Phase 14 — Provider & Auth (cluster A)

#### T1 — Credential store model `grilling` · resolved 2026-07-11
**Question.** What is the credential-store contract (read/write/probe/delete +
redaction label) such that (a) `opi doctor`'s `CredentialProbe`
(`doctor.rs:602`) reports `keyring <provider>` presence **without reading the
secret**; (b) a new `AuthDescriptor::StoreCredential` variant integrates with
`dispatch_stream`'s auth-gate (`provider_collection.rs:343`); (c) `/login`
writes and `/logout` deletes are atomic under cross-process locking; and (d) a
missing/unlocked keychain degrades to the existing env-var path with a clear
diagnostic rather than a fatal?
**Verified context.** Crate is **`keyring-core`** (the `keyring` crate is now a
samples crate — using it would pull samples, not the library). Cross-process
file lock: **`fs4`** (rustix-based, async-capable, no `unsafe` — preferred over
`fd-lock`, which uses `unsafe` on Windows and conflicts with CLAUDE.md's
avoid-unsafe posture; `fs2` is stale, exclude). In-memory secret wrapping:
**`secrecy`** (RustCrypto `Zeroizing`), distinct from `keyring-core`
(persistence) and `fs4` (locking). Linux Secret Service requires a running agent
(gnome-keyring/kwallet) — headless/SSH/CI boxes lack it, so the encrypted-file
fallback is **load-bearing, not optional**. Touch points: `doctor.rs:619-628`
match arms (currently `EnvApiKey`/`StaticApiKey` only — must gain a store arm or
silently mis-report); `AuthDescriptor` is `#[non_exhaustive]`
(`provider_collection.rs:97-119`) so a new variant is additive but every match
gains a wildcard arm.

**Resolution (2026-07-11, grilling D1–D5).**
- **Backend (D1).** OS keychain (`keyring-core`) primary for **all** persisted credentials (API keys + OAuth tokens); **no opi-managed plaintext credential file** (security improvement over pi's `auth.json`). Headless/no-daemon: API keys fall back to **env-var** (existing path + diagnostic); **OAuth login requires a keychain** (refresh tokens must persist; headless users use env API keys). This **supersedes the verifier's "encrypted-file fallback" suggestion** — env-var-only fallback sidesteps the encryption-key bootstrap problem (no OS keychain ⇒ nowhere safe to store an encryption key on headless).
- **Contract (D2).** trait `CredentialStore` + `Credential` type in **opi-ai** (abstract, no IO/env); impls (`KeychainCredentialStore` via `keyring-core`, `EnvCredentialSource`, composing `CredentialResolver`) in **opi-coding-agent** (owns IO + env, per invariant). Async methods (forward-compat with T2 OAuth refresh-on-read): `read(provider_id) -> Option<Credential>`, `write(provider_id, cred) -> Result<()>`, `delete(provider_id) -> Result<()>`, `probe(provider_id) -> CredentialSource`. Return type `enum Credential { ApiKey(SecretString), OAuthToken { access: SecretString, refresh: SecretString, expires_at: Option<DateTime<Utc>> } }`.
- **AuthDescriptor (D3).** Additive variant `AuthDescriptor::StoreCredential { key: String, display_source: String }` (cheap, Clone, **no secret**). Consumed in **non-live** paths: `doctor` `CredentialProbe` (`doctor.rs:619-628`, via `store.probe`), `dispatch_stream` status gate (`provider_collection.rs:337-350`), `--list-models` metadata — each gains a match arm. Three-state `enum CredentialSource { Present(label) | Absent | BackendUnavailable(reason) }` so doctor distinguishes "entry missing" from "no keyring daemon" for headless diagnostics. **Live-path credential injection (route-through-`dispatch_stream` vs concrete-provider-holds-`AuthSource`) is T2 — not decided here.**
- **Locking (D4).** `fs4` advisory lock, **single global** file `<user_config_dir>/opi/credential.lock` (lock file holds **no secret** — pure coordination, since the OS keychain has no cross-"read→refresh→write" transaction). Exclusive lock around **every** store mutation (`write`/`delete` here + OAuth refresh in T2), applied uniformly; **acquire-then-re-read** (pi `auth-storage.ts:505-516` pattern — concurrent opi instances share one op); short timeout + diagnostic on contention, not infinite block.
- **In-memory & redaction (D5).** `secrecy::SecretString` on all `Credential` fields; `expose_secret()` **only** at the narrow concrete-provider HTTP boundary; zeroize on drop. Scope = **store/resolver boundary (T1)**, **not** an end-to-end `SecretString`-through-provider-construction refactor (deferred follow-up). Secrets never formatted into loggable strings; `doctor`/list show only `display_source`; raw credential values are **not** registered as `SecretRedactor` patterns (avoids expanding the secret's in-memory footprint; existing Phase 7 redactor handles transport-level redaction).
- **Scope boundary.** T1 delivers the store primitives (`read`/`write`/`delete`/`probe`) + lock. The `/login`//`/logout` **command UX and OAuth flow are T2**, built on T1's locked `write`/`delete`.

**Deferred follow-up (not a ticket).** End-to-end `SecretString` through concrete-provider construction (the D5 scope cap).

#### T2 — OAuth architecture & per-request auth re-resolution `grilling`+`prototype` · resolved 2026-07-11
**Question.** The live run path **bypasses** `ProviderCollection` today
(`agent_loop.rs:118` calls `context.provider.stream` directly; the collection is
used only for `--list-models`/metadata). Which re-routing does per-request auth
RE-resolution require — **(a)** move live dispatch to
`AgentLoopContext.collection.dispatch_stream` (re-introduces the collection onto
the run path, matches pi's per-request auth model, touches `agent_loop.rs:118` +
`loop_types.rs`), or **(b)** keep `Box<dyn Provider>` but make the concrete
provider hold an `AuthSource` that re-resolves on each `stream()`? For the
chosen option: where does the OAuth token-refresh lock live (`fs4`), **how is
revocation of an in-flight token surfaced to the running session**, and **what is
the session-interaction behavior when a credential is needed mid-run**?
**Verified context.** v1 providers = Anthropic, GitHub Copilot, OpenAI Codex
(pi's 3, `oauth/index.ts:42`). PKCE authorization-code + local callback server
on `127.0.0.1` (pi `anthropic.ts:33`). `opi-spec.md:1641` **mandates** OAuth
entry conditions including revocation + session-interaction — currently omitted,
must be in T2 or Phase 14 violates its own spec. Also decide (in/out of Phase
14): `ANTHROPIC_OAUTH_TOKEN` env precedence over `ANTHROPIC_API_KEY` (pi
`env-api-keys.ts:71`); 5-minute token-expiry skew (pi `anthropic.ts:225`); manual
code/URL-paste login fallback for headless/no-browser; per-call
apiKey/headers/env override (pi `ApiStreamOptions`, realign:61) — distinct from
per-request re-resolution of the stored credential.

**Resolution (2026-07-11, grilling D1–D8).**
- **Routing (D1).** Each concrete provider holds an injected `AuthSource` enum `{ Baked(SecretString) | Store(Arc<dyn CredentialStore>, provider_id) | EnvOAuthToken }` and resolves per `stream()` (Baked returns directly; Store calls `store.read` incl. locked refresh; EnvOAuthToken uses the env token). **Provider trait + live path unchanged**; `ProviderCollection` stays off the live path (Phase 10 boundary preserved); refresh+lock centralized in the store. **Resolves the live-path routing fork T1 deferred.**
- **Scope (D2).** All 3 OAuth providers (Anthropic + GitHub Copilot + OpenAI Codex), full pi parity. Trait handles PKCE auth-code (Anthropic/Codex) + device-code (Copilot) from the start.
- **Trait (D3).** `trait OAuthProvider` + `OAuthCredential` in **opi-ai** (abstract); 3 impls + `OAuthProviderRegistry` (`register_oauth_provider`, 3 built-ins) in **opi-coding-agent**. Native async fn in trait (edition 2024, no async-trait crate). Methods `id()`, `async login(presenter) -> OAuthCredential`, `async refresh(refresh) -> OAuthCredential`. `OAuthCredential { access: SecretString, refresh: SecretString, expires_at: Option<DateTime<Utc>>, base_url: Option<String> }` (base_url for Copilot enterprise). Flow specifics (PKCE callback server vs device-code polling) live inside each impl's `login()`; trait is flow-agnostic.
- **Refresh (D4).** Double-checked locking: fast path reads keychain **without lock**; if `now + 5min < expires_at`, return (common case, zero lock cost). Refresh path: acquire `fs4` lock → re-read (double-check) → if still expired, `oauth_provider.refresh()` HTTP **under lock** → write → release. **5-min skew**; **re-read-after-failure**; refresh-HTTP timeout + T1 lock short-timeout (prevent hung-refresh deadlock). Lock-during-HTTP chosen to prevent refresh-token-rotation double-refresh races.
- **Revocation & session-interaction (D5, spec-mandated).** `/logout` = `store.delete` (T1, locked). External revocation: auth-invalid classified **non-retryable** (extend `ProviderError::is_retryable`, `provider.rs:152`) → `CredentialRevoked` diagnostic → stream errors out, session stops gracefully. Credential-needed mid-run: interactive (TUI) → `CredentialNeeded` → `LoginPresenter` prompts `/login` → resume on success (cancelable); non-interactive (RPC/JSON) → `CredentialNeeded` diagnostic + login URL → **fail the turn** (no auto-prompt, no block). **No auto-relogin mid-stream** — revocation 401 stops the turn + prompts re-login.
- **LoginPresenter (D6).** `trait LoginPresenter` in opi-ai; impls (`TuiLoginPresenter`/`RpcLoginPresenter`/`NonInteractiveLoginPresenter`) in opi-coding-agent. Methods `present_auth_url` / `present_device_code` / `await_manual_code` (manual-paste fallback) / `notify_success` / `notify_failure`. **Every OAuth flow supports the manual fallback** (headless/SSH/no-browser), mirroring pi `onManualCodeInput`.
- **ANTHROPIC_OAUTH_TOKEN (D7).** Recognized as `AuthSource::EnvOAuthToken` (no refresh token ⇒ non-refreshable, use until 401 ⇒ re-login); precedence over `ANTHROPIC_API_KEY`. In Phase 14.
- **Per-call override (D8).** **OUT of Phase 14** (→ fog) — no multi-tenant use case; `AuthSource` per-stream resolution suffices; T3 `Request` scalars cover headers/timeout; per-call `apiKey` override (pi `ApiStreamOptions`) deferred.

**Spec entry-condition check (`opi-spec.md:1641`):** credential store ✓(T1), redaction ✓(T1), doctor ✓(T1), session-interaction ✓(D5), login UX ✓(D3+D6), refresh ✓(D4), revocation ✓(D5) — all seven mandated items now designed; Phase 14 OAuth clears its own spec entry condition.

#### T3 — opi-ai Request enrichment `grilling` · resolved 2026-07-11
**Question.** Decompose into sub-decisions and decide which are in-Phase-14 vs
deferred, and in what order:
- **3a** additive `Request` scalars (`timeoutMs`/`headers`/`maxRetries`/
  `maxRetryDelayMs`/`cacheRetention`/`sessionId`) — safe plain-struct fields
  (`provider.rs:30`), coexist with the existing `CancellationToken` via a
  `select` race against `tokio::time::sleep`. **Low-risk, additive.**
- **3b** Anthropic `cache_control:{type:ephemeral,ttl}` markers gated by a
  capability flag — but the Anthropic provider has **no compat struct today**
  (`anthropic.rs` is flat; `CompatConfig` is OpenAI-Chat-only). Requires a **new
  `AnthropicCompatConfig`/`ModelCapabilities`** surface mirroring pi
  `supportsLongCacheRetention`/`supportsCacheControlOnTools` (`types.ts:471-576`).
  **Non-trivial new surface, not one flag.**
- **3c** `cacheWrite1h` + reasoning-token extension of `Usage` (`stream.rs:32`)
  and `CostBreakdown` (`stream.rs:185`) — additive; preserve opi's
  compute-separately model (cost via `calculate_cost`, not stored on the message).
- **3d** `onPayload`/`onResponse` hooks — **separate Rust-native design** (trait
  object on a `RequestContext`, or an `mpsc` channel, or an `AgentHooks`
  extension) because closures (`Box<dyn Fn>`) cannot live on a serde-derived
  `Request` without breaking `Send+Sync`. **Co-design with T10's provider
  request/response hook track.**
- **3e** dynamic `refreshModels` as an async trait method updating the live
  catalog (replaces the `Ok(())` no-op stub at `provider_collection.rs:374`).

**Verified context.** `Request` has exactly 10 fields, no per-request knobs
(`provider.rs:30-41`); no Anthropic cache_control markers anywhere today (only
OpenAI `prompt_cache_key` via `cache_key`, `openai_chat.rs:995`); `Usage`
captures only 4 token counts (realign:175). 3a/3c/3e are low-risk additive; 3b
and 3d are the risky ones — do not bundle them as a uniform "enrichment".

**Resolution (2026-07-11, grilling D1–D3).**
- **Scope (D1).** 3a + 3b + 3c + 3e IN Phase 14; **3d (onPayload/onResponse streaming hooks) deferred** (not blocking T2 — auth needs met by 3a; related to but distinct from T14's turn-level provider hooks: streaming per-chunk vs turn-level). Functional parity (3a-3e all designed), 3d implementation deferred (→ fog).
- **3a — Request scalars (`provider.rs:30`).** Add 4 wire-consumed fields: `timeout: Option<Duration>`, `extra_headers: HeaderMap`, `cache_retention: Option<CacheRetention>` (feeds 3b), `session_id: Option<String>`. Additive, default None/empty, backward-compat. `timeout` coexists with `CancellationToken` via select race. **`maxRetries`/`maxRetryDelay` NOT on Request** — retry policy is opi-agent `agent_loop`'s, not opi-ai wire.
- **3c — Usage/CostBreakdown (`stream.rs:32/185`).** `Usage += cache_write_1h_tokens: Option<u64>, reasoning_tokens: Option<u64>`; `CostBreakdown += cache_write_1h, reasoning` cost. Additive (Option for non-reporting providers), `Copy` preserved, compute-separately model kept.
- **3e — refreshModels.** `Provider` trait gains `async fn refresh_models(&self) -> Option<Vec<ModelInfo>>` with default `None` (static providers); dynamic providers override. `ProviderCollection::refresh` (`:374`, currently no-op) fans out to Some-returning providers.
- **3b — Anthropic cache_control + ModelCapabilities (D3).** New **`ModelCapabilities`** struct (`#[non_exhaustive]`) on `ModelInfo` (cross-provider): `supports_cache_control`, `supports_long_cache_retention`, `supports_thinking`, … — **not** a per-provider `AnthropicCompatConfig` (cache_control is a model capability, not a wire-compat quirk). Anthropic provider checks `model.capabilities.supports_cache_control`; emits `cache_control:{type:ephemeral,ttl}` on system prompt + last user/assistant text + last tool def (pi pattern, `anthropic-messages.ts:56-68`); `ttl='1h'` when `supports_long_cache_retention && request.cache_retention==Long`, else default ephemeral. Built-in Anthropic `ModelInfo` carry known capabilities; custom/unknown models default off (safe — no markers). OpenAI `prompt_cache_key` (`openai_chat.rs:995`) unchanged.

### Phase 15 — Safety & Sandbox (cluster K, + cluster L Operations)

#### T4 — OS-native sandbox `research`+`grilling` · resolved 2026-07-11
**Question.** What is opi's per-platform OS-native sandbox design, including a
threat model, the surfaces in scope, and a graceful-fallback policy?
- **Linux:** `seccompiler` (pure-Rust, **pre-selected** over `libseccomp-rs` to
  match the rustls-over-OpenSSL no-C-dep convention; `extrasafe` as the ergonomic
  declarative alternative) for syscall filtering, **composed with** the `landlock`
  crate (published name is **`landlock`**, NOT `landlock-rs`; MSRV 1.71, kernel
  5.13+) for FS/path restrictions. seccomp does syscalls; Landlock does paths —
  need both.
- **Windows:** `win32job` (safe Job-Object wrapper; Cargo itself uses Job Objects
  for child lifecycle) with `KILL_ON_JOB_CLOSE` set and `BREAKAWAY_OK`
  intentionally unset. **Note:** Job Objects give kill-on-close + UI/CPU/memory
  limits but **not a filesystem sandbox** — Windows path safety stays on the
  existing `WorkspaceOnly` policy.
- **macOS:** `sandbox-exec` with a Seatbelt **`.sb`** Scheme-dialect profile (NOT
  a plist), via `std::process::Command` (or the `sandbox-run` crate). Soft-deprecated
  since macOS Sierra (2016), still functional on macOS 15, no removal date —
  **rank macOS the highest-fallback-probability target**.
- **Threat-model split (load-bearing):** distinguish **(a)** confining the
  bash/tool/adapter **subprocess tree** (in-scope; where Job Object / seccomp /
  Landlock apply) from **(b)** confining **opi itself** (OUT OF SCOPE for an
  in-process T4 — needs an external runner like Docker/Gondolin; pi
  `security.md:35` warns "a partial in-process sandbox would be easy to
  misunderstand as a security boundary"). Do not market the in-process sandbox as
  a full security boundary.
- **Surfaces in scope:** name which unconfined surfaces Phase 15 covers — bash
  subprocess trees (`bash.rs:84`), write/edit FS ops, adapter children
  (`adapter_host.rs:166`) — Job Object covers bash+adapter subprocesses, Landlock
  covers FS paths for opi + children, seccomp covers syscalls process-wide.
- **Fallback policy (first-class, security-relevant):** fail-open-with-diagnostic
  vs fail-closed when the kernel mechanism is unavailable (Landlock < 5.13,
  seccomp arch edge cases, Job Object edge cases, sandbox-exec deprecation/MDM).
- **CreateProcess-suspended → Assign → Resume** race handling (Windows).

**Verified context.** opi has no sandbox today; `before_tool_call` fails open
(`hooks.rs:84`); bash runs unconfined (`bash.rs:84`); adapters spawn unconfined
(`adapter_host.rs:166`). Reconcile with ADR-019 (allowlists/visibility/hooks over
core permission profiles) and `opi-spec.md` §13.

**Resolution (2026-07-11, grilling D1–D8; spawn-surface audit `wf_d3373e28-329`).**
- **Posture (D1).** Confine the **bash subprocess tree only** — opi itself stays unconfined. Positioned as **opt-in defense-in-depth, explicitly NOT a security boundary** (untrusted code ⇒ container/VM, pi parity). More justified for opi than pi because opi's adapters are native child processes (higher blast radius than pi's in-process jiti), though T4 confines only bash in Phase 15 (D7). ADR-019 (permission-*popup* subsystem) is not in conflict — different concern.
- **Layers (D2).** L0 baseline (subprocess-tree lifecycle) **default-on**; L1 (FS) + L2 (network) + L3 (syscall) opt-in under `strict`, **default off**. Per-platform matrix: Linux L0+L1+L2+L3; macOS L0+L1+L2 (no L3); Windows L0 only.
- **Linux mechanism (D3).** `extrasafe` 0.5.1 (builds on `seccompiler`, `#![deny(unsafe_code)]`, no C deps) + `landlock` 0.4.5 (MSRV 1.71, kernel 5.13+ FS / 6.2+ net; 4 contained unsafe syscall wrappers). Applied in the child's `pre_exec` on the bash spawn: **parent pre-builds the ruleset/BPF, child runs only the raw apply syscalls** (`landlock_restrict_self`, `seccomp(SECCOMP_SET_FILTER)`) — both async-signal-safe, satisfying the `pre_exec` contract. **No `unsafe` block in opi code** — the only unsafe is the irreducible std `pre_exec` *contract*, delegated to audited libs. Rejected: helper-binary (ships a 2nd artifact across 6 release targets) and `bwrap` (external runtime dep, violates the Rust-native posture).
- **Windows mechanism (D4).** L0 only — `win32job` 2.0.3 with `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`, `JOB_OBJECT_LIMIT_BREAKAWAY_OK` **unset** (deny escape via `CREATE_BREAKAWAY_FROM_JOB`), assign-after-spawn via `child.raw_handle()` for bash + adapters. **Accept the small grandchild-escape race** (std/tokio discard the thread handle ⇒ can't `ResumeThread` a `CREATE_SUSPENDED` child; hard guarantee needs reimplementing spawn via `CreateProcessW` — disproportionate for a non-security-boundary net) — documented as residual. No resource caps in Phase 15. `strict` degrades to L0 with a startup diagnostic.
- **macOS mechanism (D5).** L1/L2 via **direct `sandbox-exec -p <templated profile>`** (inherently child-only — sandbox-exec IS the helper, no `pre_exec`/`unsafe`); L0 = `process_group(0)`. Profile is a **deny-only overlay** on the allow-all default: `(deny file-write* (subpath "/"))` with workspace+temp exceptions + `(deny network*)`. **Avoid the `sandbox-run` crate** (provenance unclear — linked repo is a PHP toolchain manager, 631 downloads). Highest-fallback-probability target (deprecated since Sierra 2016, still functional on macOS 15, no removal date; MDM-block unverified) ⇒ degrade to L0 + diagnostic when unavailable.
- **Fallback (D6).** **Default fail-open-with-diagnostic** (a configured layer can't engage ⇒ proceed at the engaged baseline); opt-in **`require = true` ⇒ fail-closed** (abort the turn — for CI/untrusted use; composes with T6). Permanent platform gaps (Windows L1–L3, macOS L3, seccomp unsupported arches) emit a **one-time startup diagnostic**, not per-command. New codes `SandboxDegraded { layer, reason }` / `SandboxUnavailable` in the existing unified Diagnostic model (additive, no schema break).
- **Surfaces (D7).** **`strict` confines bash only** (primary LLM-driven arbitrary-command surface, `bash.rs:86`). **Adapters get L0 only** (`process_group(0)` already present; Job Object is D4's add) — adapter `strict`-confinement **deferred** until a per-adapter capability-declaration mechanism exists (T5/T6 territory; the sandbox *mechanism* is generic, so wiring it to adapters is a later config step). **In-process file tools (read/write/edit/...) are NOT a T4 surface** — they do I/O in-process, stay on `PathPolicy` (T5 revisits). **`bedrock credential_process` (`credentials.rs:332/337`) OUT** — config-driven not LLM-driven; config-trust issue for T6. Verified spawn-audit (`wf_d3373e28-329`, completeness COMPLETE): opi has **no subagent/Task tool** (8 built-in tools only; nested-agent exists only as an in-process extension example, `tests/sub_agent_example.rs`, explicitly non-core per `CLAUDE.md:478-480`); `opi-tui`/`opi-agent` have zero spawn sites; no stdio-MCP-server launch (the adapter `process-jsonl` protocol is the only external-process surface besides bash).
- **Config + seccomp shape (D8).** `[sandbox] mode = "off"|"strict"` (default `off`; **`off` still ships L0** — the `process_group`/Job fix is an always-on correctness baseline, not opt-in), `require = false` flag, optional per-layer toggles (`fs`/`network`/`syscalls`), `--sandbox off|strict` CLI override (mirrors `--allow-mutating`). **L2 = INET-block with AF_UNIX carve-out** (block `socket()`/`connect()`/`sendto()`/`recvfrom()`/`accept()`/`bind()` arg-filtered on domain; AF_UNIX local IPC survives, INET/INET6/NETLINK denied). **L3 = danger-blocklist, NOT strict allowlist** (deny kernel-handle escapes: `open_by_handle_at`, `bpf`, `perf_event_open`, `ptrace`, `kexec*`, `reboot`, `init_module`/`finit_module`/`delete_module`, `swapon`/`swapoff`, `iopl`/`ioperm`, `acct`, `settimeofday`; allow `clone`/`unshare` — user-namespace unshare is more confinement, not escape).

**Scope boundary.** T4 delivers the OS-native subprocess sandbox for bash (L0 baseline + opt-in L1/L2/L3 `strict`) across Linux/macOS/Windows, the `[sandbox]` config + `--sandbox` flag, the fallback diagnostics, and the L0 `process_group` correctness fix for bash. The Phase 15 design doc and the §15 spec rewrite are NOT T4.

**Residuals / follow-ups.**
- Adapter `strict`-confinement deferred (→ fog: per-adapter capability-declaration schema; depends on T5 + T6).
- Windows hard no-escape via direct `CreateProcessW` (D4 option b) — deferred follow-up, not Phase 15.
- `credential_process` config-trust — cross-ref for T6.
- §15 spec rewrite (Phase 14 = Provider/Auth; add Phase 15/16/17) when the Phase 15 design doc lands → triggers phase4 + phase6 ledger re-sync (see memory `spec-edit-breaks-phase4-ledger`).
- Phase 15 design doc deferred until T5 + T6 resolve.

#### T5 — per-tool Operations seam `grilling` · resolved 2026-07-11
**Question.** How is a per-tool `Operations` trait (after pi's
`BashOperations`/`WriteOperations`/`ReadOperations` seam — "override these to
delegate command execution to remote systems, e.g. SSH", `bash.ts:52`)
introduced — **defined in `opi-agent`** (like `PathPolicy`), with concrete
local/sandboxed/remote impls **injected from `opi-coding-agent`** (preserving
the construction-ownership invariant) — composing with existing Phase 3/11
`PathPolicy` + mode-aware tool policy **and** the Phase 15 sandbox? Specifically:
does `Operations` layer **below** `PathPolicy`/mode-gating, or **replace** the
local FS calls `PathPolicy` currently guards? (If a remote backend takes over
write/edit, `PathPolicy`'s symlink-escape detection at `tool/mod.rs:83` becomes
meaningless — must `PathPolicy` be enforced locally before delegating, or does
the `Operations` contract inherit `PathPolicy` obligations?)

**Resolution (2026-07-11, grilling D1–D6; verified against pi's `pi-coding-agent`/`pi-agent-core` split).**
- **Contract + layering (D1).** Operations is a **pure FS/exec backend**: receives an already-resolved path, performs the op, does **no** path resolution and **no** confinement. `PathPolicy` stays the **pre-flight** (expand → canonicalize → symlink-escape → workspace-containment → resolved path) and runs **first**; Operations **layers below it**, replacing the raw `tokio::fs::*`/`Command::new` calls. Operations neither "replaces" PathPolicy nor sits beside it — it replaces the *op backend under* PathPolicy. Confinement is authoritative for the **local** backend (PathPolicy + local Ops share the same FS); for a **remote** backend, opi does only lexical normalization and the remote backend owns workspace confinement (pi parity — pi's Operations has no confinement at all).
- **Location (D2).** Operations trait + local impl + T4-sandbox wrapping live in **opi-coding-agent** (`src/tool/`, alongside `PathPolicy` and the concrete tools); opi-agent's `Tool` trait is unchanged and unaware of Operations. **Corrects the ticket premise** ("defined in opi-agent like PathPolicy") — `PathPolicy` is itself in opi-coding-agent, not opi-agent (verified: zero `PathPolicy` hits in `crates/opi-agent/`). Deciding principle: a trait lives in opi-agent only if opi-agent's runtime invokes it — `agent_loop` calls `AgentHooks`/`CompactionHooks` (so those are in opi-agent) but never Operations or PathPolicy (internal to how concrete tools execute). **pi precedent confirms**: pi splits the same way (`pi-agent-core` runtime has no `tools/` dir, only the generic `AgentTool` trait; all 7 Operations interfaces live in `pi-coding-agent/src/core/tools/`).
- **Tools in scope (D3).** Phase-15 Operations for **read/write/edit/bash**. Nav tools (grep/find/ls/glob) stay **local-walk** — their `ignore`-crate `WalkBuilder` (`nav_walk_builder`, `tool/mod.rs:199`) traverses the local FS directly and can't be cleanly redirected to a remote backend without a remote-aware walker redesign that loses gitignore semantics; pi's nav Operations are narrow and never overridden in practice (ssh/sandbox/gondolin examples override only Read/Write/Edit/Bash). Nav Operations → fog.
- **Granularity (D4).** Two grouped traits: **`FileOperations`** (read+write+edit — readFile, writeFile, mkdir, metadata, access; atomic write via temp+rename is an *implementation detail* of the impl, not the contract) + **`BashOperations`** (bash exec). A remote FS backend delegates the FS as a unit, not per-tool; bash is process-exec, orthogonal. Rejected: one-trait-per-tool (pi's model — duplicates readFile/writeFile across Read/Edit and Write/Edit; backends are full-FS anyway) and single fat trait (interface-segregation; bash-only backends e.g. the T4 sandbox shouldn't implement file methods).
- **Injection (D5).** Constructor injection — `Arc<dyn FileOperations>` → read/write/edit, `Arc<dyn BashOperations>` → bash; `build_tools` wires the local default. `Arc` (shared local impl, cheap clones, lives with the tool). Phase 15 ships the **seam + local impl only** — no SSH/container backends (future/examples, pi parity); the seam's value is enabling future remote delegation *and* giving T4's sandbox a home (D6). Testable via mock impls (mirrors `opi-ai::test_support::MockProvider`).
- **Sandbox composition (D6).** The T4 sandbox lives **inside the local `BashOperations::exec`** (driven by `[sandbox]` config; `pre_exec`/`sandbox-exec`/Job-Object applied to the `Command` before spawn). `FileOperations` is **unsandboxed** (file tools stay PathPolicy-guarded — T4 confines only the bash subprocess tree). Remote/custom BashOps **bypasses** the sandbox and owns its own confinement. The Operations seam is exactly where "local + T4-sandboxed" vs "remote" plugs in — the separation D1 established. Matches pi's model (sandbox IS a BashOperations impl, not a wrapper).

**T4 integration note.** T5 refines T4's attach point: the bash spawn moves from `bash.rs:86` into local `BashOperations::exec`, so the sandbox lives there rather than in the tool. Same logical operation (bash command spawn); the Phase-15 design doc reconciles the code location. Not a conflict.

**Scope boundary.** T5 delivers the Operations seam — `FileOperations` + `BashOperations` traits in opi-coding-agent, local impls, `Arc<dyn>` constructor injection in `build_tools`, PathPolicy confirmed as the pre-flight above Operations, and the T4 sandbox relocated into local `BashOperations::exec`. The Phase 15 design doc, remote backends, and nav-tool Operations are NOT T5.

**Residuals / follow-ups.**
- Nav-tool Operations (grep/find/ls/glob) deferred (→ fog).
- Remote backends (SSH/container) — future/examples, not Phase 15.
- Method signatures (read windowing for large files, streaming) — design-doc detail.
- Phase 15 design doc deferred until T6 resolves.

#### T6 — project-trust gate `grilling` · resolved 2026-07-11
**Question.** What is the exact scope of opi's project-trust gate — a per-cwd
`trust.json` (after pi's `ProjectTrustStore`, `trust-manager.ts:43-57` ancestor
walk + always/never/ask decision) gating **loading** of which specific
project-local resources (skills/packages/fragments/themes/`.opi` config) at which
discovery layer (user=1 / project=2 / explicit + package-composed layers), with
what override flags — and **critically: does trust-gating extend to adapter
startup?** opi adapter packages (`[adapter]`, process-jsonl) run **native child
processes** at `runtime_packages.rs:21-54` — strictly higher blast radius than
pi's in-process jiti extensions; a discovery-only gate would still let an
untrusted project's adapter run. Confirm trust gates **loading**, not tool
**execution** (pi does not gate execution).

**Resolution (2026-07-11, grilling D1–D6; verified against pi `ProjectTrustStore` / `project-trust.ts`).**
- **Scope (D1).** Trust gates **loading** of project-local resources, including (the pi-deviating key point) **project-local adapter declarations** — an untrusted project's `.opi/packages.toml` adapter entries are not loaded, so their native child processes **do not start**. Still "gate loading, not execution" in principle, but for opi's native adapters loading a declaration is coupled to spawning the child (unlike pi's in-process jiti), so opi gates it at declaration-load. **User-global** resources including user-global adapters are **ungated** (user scope, always trusted). **Tool execution** (bash/read/write/edit/grep/find/ls/glob) is **ungated** (pi parity — once in a session, tools run with the user's privileges; trust decides what project resources load, not what the model asks tools to do).
- **Context-file deviation (D2).** Project-local `AGENTS.md`/`CLAUDE.md` are **NOT auto-injected** into the system prompt for untrusted projects (deviates from pi, which loads them regardless) — they are a direct prompt-injection channel. User-global context files still load; the files remain readable via the `read` tool.
- **Store (D3).** pi-style `ProjectTrustStore` at `{user_config_dir}/trust.json` — flat `Map<canonical_path, bool>` (true/false only; absent = no decision = ask), ancestor walk (`Path::parent()`, nearest true/false wins), `std::fs::canonicalize` realpath key, `fs4` sidecar `trust.json.lock` (T1 consistency; acquire-then-reread). Concrete struct in **opi-coding-agent** (no opi-agent trait — the store is consulted at session-start in opi-coding-agent, not by opi-agent's runtime, per the T5 deciding principle). No schema version/metadata in Phase 15.
- **Ask UX (D4).** Full TUI trust prompt (Trust / Trust-parent / Trust-session / Deny / Deny-session, mirroring pi) + a new `AppState::AwaitingTrust` variant, fired on first cd into a project with **trust-requiring resources** (mirror pi's `hasTrustRequiringProjectResources` — a bare `.opi` dir doesn't count; only prompt when there's `.opi` config/skills/fragments/themes/extensions/project packages/project `AGENTS.md`). Non-interactive/RPC **can't ask ⇒ default untrusted** (skip project resources) unless `--trust` or `default_project_trust=always` (mirror pi's "ask + no-UI → false").
- **Overrides (D5).** `--trust` / `--no-trust` flags (clearer than pi's `--approve`); `[defaults] default_project_trust = "ask"|"always"|"never"` (**global-only** — structurally required: the setting must load before trust resolution and project `.opi/config.toml` is itself gated/not-yet-loaded; default `ask`); pi-style precedence (CLI → extension event → store → default → ask); `/trust` slash command to change trust mid-session; no env var in Phase 15.
- **Extension hook (D6).** `project_trust` extension hook (mirror pi) — only **user-global + CLI `-e`** resolvers may own the decision (structurally enforced: project `.opi/extensions` aren't loaded until after trust resolves), with a **minimal pre-trust UI surface** (`select`/`confirm`/`input`/`notify` — no full TUI access, preventing privilege escalation before consent). First yes/no wins; `Undecided` falls through. **opi-coding-agent-local trait** (not on opi-agent's `Extension` trait — opi-agent's runtime never invokes it; trust resolution is session-start, coding-agent-local). The ADR-019-consistent seam for custom trust policy (signed manifests, team allowlists, org policy).

**Gated list (untrusted ⇒ skip):** `.opi/config.toml`; `.opi/{extensions,skills,fragments,themes}`; project `AGENTS.md`/`CLAUDE.md` (D2 deviation); project `.opi/packages.toml` adapter declarations (D1).

**Not gated (always loaded):** user-global config/extensions/skills/fragments/themes/adapters/context-files; CLI `-e` extensions; all tool execution. (opi has no project-config auto-install, so no auto-install gate — simpler than pi.)

**T4 cross-ref resolved.** T4 flagged `bedrock credential_process` as "config-trust for T6" — T6's `.opi/config.toml` gating covers it: an untrusted project's config-set `credential_process` is never loaded, never executed. User-config `credential_process` is user-trusted. T4's concern closes here.

**Scope boundary.** T6 delivers the project-trust gate — `ProjectTrustStore`, the gated/not-gated resource lists, the TUI ask prompt + `AppState::AwaitingTrust`, `--trust`/`--no-trust` + `default_project_trust` + `/trust`, the `project_trust` extension hook, and adapter-declaration gating. The Phase 15 design doc and §15 spec rewrite are NOT T6.

**Residuals / follow-ups.**
- New TUI trust-prompt widget + `AppState::AwaitingTrust` is real opi-tui work.
- The "trust-requiring resources" detector (mirror pi).
- **Phase 15 now fully resolved (T4+T5+T6).** The Phase 15 design doc (`docs/superpowers/specs/YYYY-MM-DD-phase15-safety-sandbox-design.md` — synthesizes T4 sandbox + T5 Operations + T6 trust) is the next deliverable, followed by the §15 spec rewrite (triggers phase4 + phase6 ledger re-sync per memory `spec-edit-breaks-phase4-ledger`).

### Phase 16 — Agent Intelligence (clusters D + E)

#### T7 — Skills/templates runtime `grilling`+`prototype` · resolved 2026-07-11
**Question.** How do we add production skill/template runtime to
`opi-coding-agent` (opi-agent stays skill-free) covering: **(a)** `/skill:<name>
[args]` slash dispatch in `interactive.rs` that reads `SKILL.md` and injects an
`<skill>` block; **(b)** `SystemPromptBuilder` emission of an
`<available_skills>` XML section (name/description/location + read-tool prose,
gated on the read tool being selected) and **wiring the existing-but-bypassed
`disable_model_invocation` flag** (`skill.rs:105,446-451` — currently dead code)
into that emission; **(c)** bash-style positional argument substitution
(`$1`/`$@`/`$ARGUMENTS`/`${N:-default}`) replacing the unused `{{name}}`
`expand_fragment_body` path; **(d)** recursive `.gitignore`/`.ignore`/`.fdignore`-aware
discovery via the **`ignore`** crate (new `[workspace.dependencies]` entry,
replaces the single-level `discover_skills` scan at `skill.rs:341-366`)?
**Verified context.** opi skills are package-discovery only today; **bodies are
discarded** by the harness (`harness.rs:2041-2050`, metadata-only); no `/skill:`
dispatch, no production argument substitution, no model-invocable XML. opi-agent
has zero skill/template code. `disable_model_invocation` + `expand_fragment_body`
already exist as dead code — the work is "wire up existing unused mechanisms +
add the missing XML/dispatch/recursion", not greenfield.

**Resolution (2026-07-11, grilling D1–D7; adversarial verification `wf_12096538-a73`, 5 verifiers — all claims hold or corrected below).**
- **Split (D1).** Skills and fragments stay two distinct user-facing mechanisms (pi's model); `skill.rs` and `prompt_fragment.rs` are already separate modules with distinct types (`SkillManifest`/`SkillResource`/`SkillRegistry` vs `FragmentManifest`/`FragmentResource`/`FragmentRegistry`/`FragmentArgument`).
- **Skill body/args (D2).** `/skill:name args` loads the SKILL.md body, wraps as `<skill name location>\nReferences relative to {baseDir}.\n\n{body}\n</skill>`, appends raw args verbatim. **No substitution** on the body. Verified against pi: `agent-session.ts:1273-1274` builds the block then `args ? \`${skillBlock}\n\n${args}\` : skillBlock` — append-only; bash-style `$1`/`$@`/`$ARGUMENTS`/`${N:-default}` lives only in `prompt-templates.ts:60-101` `substituteArgs`, applied to TEMPLATE bodies, never skills. **Corrects T7(c)**'s "bash-style for skills" (that's pi's template mechanism).
- **Fragment args (D3).** Fragments keep opi's existing `{{name}}` declared-arg engine — wire the dead `expand_fragment_body` (`prompt_fragment.rs:504`) + `FragmentRegistry::expand` (`:583`); `FragmentManifest.arguments` stays; missing-required → `FragmentDiscoveryError::MissingArgument`. Verified zero production callers (only `tests/prompt_fragments.rs`). Rust-native deviation from pi-template syntax per §3.2.
- **Discovery (D4).** Split: skills **recursive + `.gitignore`/`.ignore`/`.fdignore`** via the existing `ignore = "0.4"` workspace dep (`Cargo.toml:65`, `opi-coding-agent/Cargo.toml:28` — verified NOT new; **corrects T7(d)**'s "new dep" premise); fragments stay **single-level** (pi templates are non-recursive). Verified current `discover_skills` (`skill.rs:329-372`) and `discover_fragments` (`prompt_fragment.rs:414-455`) are both single-level today. **No symlink-follow** (opi safety default; deviates from pi). Keep opi's looser `[a-z0-9-]` name rule (no pi `name==dirname` requirement).
- **System prompt (D5).** `SystemPromptBuilder` emits a pi-style `<available_skills>` XML block (name/description/**location**=absolute SKILL.md path + read-tool prose) for skills, **replacing** the inert prose "Discovered skills" list currently concatenated into `context_files` (verified: `harness.rs:800-809` feeds `format_for_system_prompt()` — incl. the "Discovered skills"/"Discovered prompt fragments" sections at `:201-206` — into `builder.context_files()`; the V4 verifier's "no skill listing today" was a misread of the `context_files` *module* vs the builder setter). Read-tool-gated (verified implementable: tools built at `harness.rs:762` before the builder runs; read-tool name is exactly `"read"`). `disable_model_invocation=true` skills excluded from the XML but still `/skill:name`-dispatchable (the parsed-but-bypassed flag — `auto_invocable` `skill.rs:446-451` has zero production callers — finally takes effect). **Fragments dropped from the system prompt** (pi-template parity — `system-prompt.ts` never references templates; stay reachable via RPC metadata + `/help`).
- **Dispatch (D6).** `/skill:name args` + `/fragment:name args` as **one text-expansion function** applied at every text-entry point: interactive prompt, non-interactive prompt, and the steer/follow-up queues. The `/fragment:` colon prefix is chosen for **consistency with `/skill:`** and to avoid collision with opi's existing session slash-command set (`/model`, `/session`, `/name`, `/branch`, `/tree`, `/image`, …); bare-name fragment dispatch (pi's template UX) is **deferred** (would need a session-command-precedence rule). **Steer/follow-up expansion site (verified):** `harness.steer`/`harness.follow_up` (`harness.rs:1736-1743`) currently forward raw `String` to opi-agent — they gain expand-then-forward; and the RPC running-mode branch `self.control.steer/follow_up` (`rpc.rs:650/658`) currently bypasses the harness, so it must call the expansion helper before delegating (or route through `harness.steer/follow_up`). Mirrors pi's coding-agent-layer expansion (`agent-session.ts:1294-1325`). RPC gets **text-level expansion only** in Phase 16 — a dedicated `SdkCommand::Skill`/`Fragment` pulls T11 forward, so defer to Phase 17.
- **Failure handling (D7 — deliberate opi divergence from pi).** Mode-sensitive: interactive → unknown skill/fragment name = diagnostic error, do **not** forward literal `/skill:typo` to the model; non-interactive/RPC/steer → pass original text through + diagnostic. pi uniformly passes unknown names through in all modes (`agent-session.ts:1268,1282`); opi's interactive-error is a **deliberate divergence** for typo UX. Missing-required-fragment-arg → diagnostic everywhere + fail expansion (no half-expanded body). Body-load IO failure → diagnostic + fail.

**Invariant (verified `wf_12096538-a73`).** All skill/fragment logic lives in **opi-coding-agent** (dispatch, expansion, discovery, `<available_skills>` emission in `prompt.rs`); **opi-agent stays skill-free** — `crates/opi-agent/src` has zero `skill`/`prompt_fragment`/`SkillRegistry`/`FragmentRegistry` references. The steer/follow-up queues are raw `String` (`loop_types.rs:47-49`) drained inside `agent_loop.rs:552-584`; opi-agent has no expansion hook and needs none, because every queue-feeding call site (`harness.rs:1736-1743`, `rpc.rs:648-663`) is in opi-coding-agent and expands before delegating. The system prompt built at `harness.rs:811` (before `Agent::new` at `:823`) reaches `agent_loop` with no opi-agent change.

**Cross-refs.** T6 (Phase 15) precondition: T6's trust gate must filter `layers.skills`/`layers.fragments` at `discover_resources` time (`harness.rs:775/2041/2057`) so an untrusted project's skills/fragments can't resolve via `/skill:`/`/fragment:`; T7 depends on Phase 15 landing first (already sequenced 14→15→16→17).

**Scope boundary.** T7 delivers the design decisions above (verified). The Phase 16 design doc (synthesizes T7 + T8 + T9), the §15 spec rewrite, and the implementation (via `opi-implement`) are NOT T7.

**Residuals / follow-ups.**
- Dedicated RPC `SdkCommand::Skill`/`Fragment` deferred to T11/Phase 17.
- Bare-name fragment dispatch (`/translate` instead of `/fragment:translate`, pi's template UX) deferred — needs a session-command-precedence rule to avoid colliding with `/model`, `/session`, `/name`, etc.
- Skill name-validation strictness kept at opi's looser rule (pi's `name==dirname` not adopted).
- Steer/follow-up expansion must cover both rpc.rs branches (`self.control.*` at `:650/658` and `harness.*` at `:652/660`) — implementation detail for the design doc.

#### T8 — LLM compaction & branch-summary generation `grilling` · resolved 2026-07-11
**Question.** How do we inject LLM-driven compaction and branch-summary into
`opi-agent` through the `CompactionHooks` trait (`compaction.rs:98-101`), with
the provider-backed hook impl in `opi-coding-agent`, covering:
- **(a)** **widen** `CompactionHooks::generate_summary` from sync
  `fn(&self, &[AgentMessage]) -> Option<String>` to async
  `Pin<Box<dyn Future<Output = Option<String>> + Send>>` (mirroring
  `transform_context`, `hooks.rs:67-73`), **and** specify how the product
  `CodingHarness` threads the provider reference into the compaction call site
  (`opi-agent`'s `CompactionEngine::compact` does not hold a provider today);
- **(b)** **rewrite** `find_split_point` (`compaction.rs:184-201`) from
  entry-count-fraction (keep last 25%, min 1) to **token-budget cut at
  user-message turn boundaries** (pi `keepRecentTokens` default 20000) —
  **breaking change**, tests pinning the 25% heuristic will change;
- **(c)** `readFiles`/`modifiedFiles` file-operation tracking on
  `CompactionEntry` (`session.rs:102-110`) — **additive on v1**, triggers the
  Phase 13 additive-entry/forward-compat policy + doc-guard tests
  (`phase13_session_context_docs.rs` pins `KNOWN_ENTRY_TYPES` at
  `session.rs:515-525`);
- **(d)** `Session::move_to(entry_id, summary)` library method on opi-agent's
  coordinator (requires moving fork-at-entry from `opi-coding-agent/session_cli.rs`
  into the opi-agent coordinator — refactor);
- **(e)** durable `custom` + `active_tools_change` `SessionEntry` variants
  (additive v1; update `KNOWN_ENTRY_TYPES` + guards)?

**Spec-drift note.** `opi-spec.md:1595-1604` names "Phase 14 terminal/product
polish" as the deferral home for branch-summary generation UX and interactive
`/export`. Under the user-approved sequence, Phase 14 is now Provider/Auth — T8
**pulls branch-summary generation forward into Phase 16**. A follow-up §15
renumbering/edit must reconcile this when the Phase 16 design lands.

**Resolution (2026-07-11, grilling Q1–Q8; 16-agent deep pass `wf_ed9ff140-12c`).**
- **Scope (D1).** (a) async LLM-summary hook + (b) token-budget split + (d) `move_to`+branch-summary generation IN Phase 16; (c) readFiles/modifiedFiles + (e) new entry variants DEFER to Phase 17. Coherent feature: "opi can LLM-summarize compactions (a+b) and generate branch summaries on `/move` (d)."
- **Signature (D2).** Widen `CompactionHooks::generate_summary` to `fn(&self, &[AgentMessage], signal: CancellationToken) -> Pin<Box<dyn Future<Output = Result<Option<String>, AgentError>> + Send>>`. Boxed future MANDATORY (dyn-compat with `&dyn CompactionHooks` at `compaction.rs:139` — native async-fn-in-trait is not dyn-compatible); `Result` honors §9.5 (`opi-spec.md:988`) visible-error-on-overflow mandate (a bare `Option` would swallow provider failures during `Overflow` and silently drop history); `Ok(None)` = hook declines → core fallback. Mirrors `AgentHooks::transform_context` (`hooks.rs:67-73`).
- **Injection (D3).** `SessionCoordinator` HOLDS `Arc<dyn CompactionHooks>` via a `.with_hooks()` builder (constructor arity unchanged → ~30 test constructions untouched); impl `ProviderCompactionHooks { provider, summary_model, thinking }` in new `crates/opi-coding-agent/src/compaction_hooks.rs`; `DefaultCompactionHooks` stays the providerless-test fallback; additive `[compaction] summary_model` config (defaults to chat model). This is the **first provider-backed hook impl in the workspace** (`CodingAgentHooks`/`InteractiveCodingHooks` override only `convert_to_llm` today, `harness.rs:2258-2282`).
- **Arc accessor (D4).** Add `Agent::provider_arc(&self) -> Arc<dyn Provider>` — `Agent` already holds the Arc internally (`agent.rs:93,119`); the accessor is a trivial `Arc::clone`, minimal cascade, `Agent::new` signature unchanged, ~15 builder sites untouched. (Rejected: widening the boundary to `Arc<dyn Provider>` at ~15 sites — speculative; defer until T2/T14 is a concrete driver. No `Arc` exists at the boundary today; `Agent::provider()` returns only `&dyn Provider`.)
- **Async cascade (D5).** `compact`/`run_compaction`/`execute_compaction`/`on_turn_end_simple` + two public fns `compact_with_diagnostic`/`compact` (`harness.rs:1851,1888`) become `async` — the latter two are a published-crate **public-API break** (CHANGELOG `Breaking Changes`); `crates/opi-agent/tests/compaction.rs` = 20 SYNC `#[test]` (premise corrected — NOT tokio) → migrate to `#[tokio::test] async` + ~16 `.await` additions across `session_runtime.rs`/`rpc_jsonl.rs`/`sdk_embedding.rs`; `on_turn_end` stays sync (pure `should_compact` check).
- **find_split_point (D6).** Add `keep_recent_tokens: u64` (default **20000**, pi-parity, spec-reserved at `opi-spec.md:869`) to `CompactionConfig` — **separate from trigger** `threshold_tokens` (100_000; premise corrected: pi `keepRecentTokens` is the *retention* budget, distinct from the trigger `contextWindow − 16384`). Walk back from tip accumulating chars/4 estimate; when budget exceeded, snap forward to the next User-message cut; always keep the last complete User-turn; **no split-mid-turn** (deliberate pi divergence — avoids doubling the provider-call surface + dodges the orphaned-ToolResult invariant risk; safe failure mode = keep the whole over-budget turn). ~9-11 test rewrites (`split_point_keeps_tail` + cluster).
- **move_to (D7).** `move_to(entry_id, summary)` on **`SessionCoordinator`** (opi-coding-agent) — **NOT** opi-agent `SessionFacade` (test-only, no compaction buffer → cannot maintain INV-4 alignment; deciding principle from T5/T6: a method lives in opi-agent only if opi-agent's runtime invokes it — `agent_loop`/`Agent` never call move_to). LLM generation in `CodingHarness::generate_branch_summary` (sibling to the D3 hook impl). `fork_session` gains `ForkTarget { ActiveTip, At{entry_id}, Before{user_msg_id} }`. opi-agent `Phase::BranchSummary` + `AgentHarness::enqueue_branch_summary` left dormant + doc comment (superseded). (Premise corrected: T8(d) is NOT a "pure refactor" — `fork_session` (`session_cli.rs:210-258`) forks the whole active chain at the tip with no `entry_id`; T8(d) is a new primitive + a new `ForkTarget` param.)
- **move_to behavior (D8).** Idempotent re-move to the current tip = **no-op** (INV-5; pi appends unconditionally — opi diverges to avoid duplicate BranchSummary/Leaf); `/move <entry_id>` + RPC `SdkCommand::Move` (picker UX → Phase 17 TUI-polish); `BranchSummaryMessage.entry_count` populated from abandoned-tail length (was the Phase-13.2 placeholder `0`); `Phase::BranchSummary` dormant + comment. Invariants: persisted `BranchSummary.parent_id == Some(entry_id)` (the NEW tip), exactly one `Leaf` per move, `reconstruct_context == new_agent_messages` (buffer alignment), total usage NOT reset.
- **Spec-drift (D9).** Retarget `opi-spec.md:1595-1598` (branch_summary *generation*) → Phase 16; keep `:1603-1604` (`/export`) → Phase 17 TUI-polish (terminal command-surface, not Agent Intelligence); §15 insert Phase 14/15/16 entries + renumber TUI-polish → Phase 17. **Batched with the Phase 16 design doc landing** (triggers phase4+phase6 snapshot + live-ledger raw-hash re-sync per memory `spec-edit-breaks-phase4-ledger`). Binding spec mandates the design doc must honor: §9.5 (`:988`) record summary source Core-vs-Hook (`SummarySource` preserved) AND visible error on overflow (D2's `Result`); `:869` `keep_recent_tokens` reserved (D6 sanctioned); `:922` `branch_summary` additive-on-v1, reused by `move_to` (no new entry type).

**Premise corrections (ticket framing errors surfaced by the deep pass).** T8(d) is NOT a "pure refactor" (fork_session forks whole active chain at tip, no entry_id; T8(d) = new primitive + ForkTarget). "opi-agent's coordinator" was ambiguous — two exist; `move_to` → `SessionCoordinator` (opi-coding-agent). No `Agent::provider_arc()` exists today (`Agent::provider()` returns `&dyn Provider`). §9.5 mandates `Result` not bare `Option`. Boxed future is mandatory (not preferential). T8(a) is the first provider-backed hook in the workspace. 20 compaction tests are sync `#[test]` (not tokio). pi `keepRecentTokens=20000` is retention, separate from trigger.

**Scope boundary.** T8 delivers the design decisions above (all verified). The Phase 16 design doc (synthesizes T7 + T8 + T9), the §15 spec rewrite, and the implementation (via `opi-implement`) are NOT T8.

**Residuals / follow-ups.**
- (c) readFiles/modifiedFiles on `CompactionEntry` deferred → Phase 17 (not substrate for a/b/d; full additive-entry ritual + 11 struct-literal edits for no Phase-16 consumer; additive-on-v1 so future revival reads old JSONL cleanly).
- (e) `active_tools_change` + `custom` entry variants deferred → Phase 17 (no producer for `active_tools_change`; `custom` would duplicate `ExtensionStateEntry` without a driver; `custom_message` is spec-deferred at `:1599-1602`).
- Full pi split-mid-turn (two LLM calls, `TURN_PREFIX_SUMMARIZATION_PROMPT`) → fog (deliberate D6 simplification).
- `/move` picker UX → Phase 17 TUI-polish (D8 ships typed-id + RPC only).
- **Phase 16 design doc unblocked** — T9 (read-tool inline image) resolved 2026-07-11; T7+T8+T9 all resolved and the design doc is draftable. §15 spec rewrite batched with it (triggers phase4+phase6 ledger re-sync).
- §15 spec rewrite batched with the Phase 16 design doc (D9) → triggers phase4 + phase6 ledger re-sync.

#### T9 — Read-tool inline image (multimodal) `grilling` · resolved 2026-07-11  · *(promoted by verification)*
**Question.** How does the `read` tool detect image MIME (pi
`detectImageMimeType`) and auto-resize (pi `processImage`) so a model invoking
`read` on an image path gets usable multimodal input instead of binary-rejected
text (`read.rs:317-327` returns text only today)? Rust-native via the **`image`**
crate; reuses opi's existing image-input attachment path (`image.rs`).
**Verified context.** realign dim 8 line 342. Bounded, substrate-ready,
high-value (today `read` on an image is useless), not ecosystem scope. Fits the
Phase 16 multimodal/intelligence theme.

**Resolution (2026-07-11, grilling Q1–Q8 + Q5′; adversarial verification `wf_a1462c1a-da9`, 4 lenses, 16 findings folded in).**
- **Delivery + wire scope (Q1).** **Inline**: `read` returns image content; the wire-layer `tool_result` image serialization fix is **in T9 scope**. *Premise correction:* the ticket's "reuse the existing image-input attachment path" was incomplete — that path is user-message-only (`image.rs` → `InputContent::Image` → `pending_images`) and **5 provider files / 3 wire patterns** degrade `tool_result` images to `[image: ...]` text today (`anthropic.rs:976-978`, `gemini.rs:589-591`, `bedrock/mod.rs:952-953`, `openai_chat.rs:1286-1288`, `openai_responses.rs:930-931`). Staged-via-`pending_images` rejected (next-turn semantics break read-then-reason).
- **Processing (Q2).** Decode + resize via the **`image`** crate (new `[workspace.dependencies]` entry, used by opi-coding-agent). Format detection folds into decode (`ImageReader::with_guessed_format` magic bytes) — no `infer`, no extension reliance. Pure-Rust, no C deps (§3.2).
- **Resize targets (Q3).** Long side ≤**1568px** (Anthropic's resample threshold; under OpenAI ~2000 and Gemini 3072), output ≤**5 MiB** (iterative downscale to fit), preserve `media_type`; GIF → first-frame PNG. Compile-time constants.
- **Integration (Q4).** Early branch in `ReadTool::execute` before `stream_read_window` via `ImageReader::open().with_guessed_format()`; image → decode/resize/re-encode → image content; else fall through to the **unchanged** text path. offset/limit ignored for images. Processing is a **tool-layer** helper; FS bytes via the T5 `FileOperations` backend. *Dependency note:* `FileOperations` is a Phase-15 (T5) substrate **not yet implemented** (zero `trait FileOperations` hits today); T9 assumes the 14→15→16 sequence — if T9 lands before T5, it uses the existing direct `tokio::fs` path (`read.rs:303`) and migrates when T5 ships.
- **Wire fix (Q5, verified).** Fix **3 fix-sites covering 4 providers** by reusing the existing base64 inline encoder (reuse source: `anthropic.rs:923-935` user-message `InputContent::Image` encoder; fix *targets*: the tool_result blocks at `anthropic.rs:970-998`, `gemini.rs:583-615` which emits key **`inline_data`** snake_case and covers Vertex via `vertex.rs:121` delegation, and `bedrock/mod.rs:944-956` — the **Converse** `toolResult` block). Degrade **2 fix-sites covering 5 OpenAI-compat providers** (`openai_chat.rs:1280-1308` covers Chat/OpenRouter/Mistral/Azure via the shared `OpenAiChatProvider`; `openai_responses.rs:924-949` is separate — confirmed API limitation: tool messages are string-content). **Backward-compat:** text-only `tool_result` stays the joined-string byte-identical shape (Phase 11.9 invariant, `anthropic.rs:987-990`); the content-block array is emitted **only when an `Image` is present** (same conditional at Gemini/Bedrock).
- **Degraded-provider placeholder (Q5′ — corrects Q5's infeasible wire-layer rich placeholder).** The wire site cannot build a rich placeholder: `OutputContent::Image` carries only `{source, media_type}` (`message.rs:119-127`) — no dims/path. Resolution: `read` emits a **companion `OutputContent::Text`** (`"image: {media_type}, {WxH}, at {path}"`) alongside the `Image`; fixed providers render a labeled image content-block array; degraded providers' `Image` arm emits `""` so their existing join-all-content behavior surfaces the rich label text. No `OutputContent` schema change; no regression (the `Image` variant has zero producers today; `image_tool_results.rs:160-183` pins the session serde shape and is untouched).
- **Config (Q6).** **Always-on, no new knob.** Reuse `defaults.max_image_bytes` (20 MiB) as the pre-decode input guard (checked before `decode()` to avoid OOM on a huge file).
- **Gating (Q7).** **Fire-and-forget**; no `supports_image_input` T3 extension. Q5′ handles the provider-wire case; text-only-on-fixed-provider is **limited to 4 Mistral models today** (`supports_images=false`, `mistral.rs:37-64`) — covered by the fog entry.
- **Errors/format breadth (Q8).** New additive `ReadFileError::ImageDecode` (`read.rs:292`, surfaces `image::ImageError`) **AND** a matching new `FsToolError` variant + diagnostic code in opi-agent (mirrors `BinaryFile` at `read.rs:157-161`) — the private `ReadFileError` alone does not reach the model. Accept the `image` crate's default decode set (PNG/JPEG/GIF/WebP/BMP/TIFF/…), re-encode to a supported `MediaType` (preserve JPEG/WebP; others → PNG). Non-image binary unchanged (NUL → `BinaryFile` fallthrough).

**Scope boundary.** T9 delivers the decisions above. The Phase 16 design doc (synthesizes T7+T8+T9), the §15 spec rewrite, and implementation (via `opi-implement`) are NOT T9.

**Residuals / follow-ups.**
- Phase 16 design doc **unblocks on enshrinement of this resolution** (T9 header → resolved; Decisions-so-far += T9; T8's BLOCKED residual struck) — then all three Phase-16 tickets (T7/T8/T9) are resolved and the design doc is draftable.
- New fog: `supports_image_input` capability + tool-level gating (graduate if a text-only opi model becomes common — Mistral today); provider-adaptive resize sizing (per-model dims).
- APNG / animated WebP silently first-frame-collapsed by the `image` crate default decode (parity with the explicit GIF→first-frame); multi-frame extraction → fog if a driver appears.
- EXIF orientation not applied (`image` crate default; needs `kamadak-exif`) — JPEGs with Orientation ≠ 1 reach the model rotated/mirrored; accept in Phase 16 or add EXIF re-orientation (design-doc decision).
- Bedrock `toolResult` image is **model-gated** (Amazon Nova + Claude 3/4 only, per AWS `ToolResultContentBlock`); non-Nova/Claude Bedrock models need degradation even though the wire supports it — conservatively degrade or gate on model id (design-doc decision).
- Responses-API (`openai_responses.rs:924-949`) degrade is conservative — re-verify against current OpenAI docs before locking (the most likely of the 5 to gain native image tool-result support).
- §15 spec rewrite batched with the Phase 16 design doc → triggers phase4 + phase6 ledger re-sync (memory `spec-edit-breaks-phase4-ledger`).

### Phase 17 — Extension surface + TUI engine (clusters C + F)

#### T10 — TUI engine foundation `prototype`+`grilling` (visual) · resolved 2026-07-11
**Question (two sub-decisions).**
- **Perf (premise-corrected).** ratatui's `Terminal::draw()` **already cell-diffs**
  and flushes only changed cells (ratatui-core `buffers.rs` flush) — opi does
  **not** send a full frame; the realign doc's "full-frame redraw" wording (lines
  352/375) is imprecise. The real cost is the **50ms busy poll**
  (`interactive.rs:402`) + **CPU-side full-Shell widget-tree recomposition** each
  tick. How do we reduce it: **(a)** replace the 50ms busy poll with event-driven
  wake (block on a crossterm event read with a timeout, drop the poll); **(b)**
  add a dirty-state flag so Shell recomposition into the buffer is skipped when
  `AppState` is unchanged; **(c)** adopt ratatui `Viewport::Inline`/`Fixed`
  where appropriate? Do **not** build a redundant custom diff layer.
- **Overlay stack.** Build a Rust-native `OverlayStack` in `opi-tui` (anchors
  `enum` `#[derive(Clone, Copy)]`, percentage `SizeValue`, focus-restore state
  machine, `OverlayHandle` as a typed index — no mature crate exists) mirroring
  pi's `showOverlay` (`tui.ts:493`, 9 anchors). What is the **minimal API** the
  command palette and T10c's overlay-dependent extension UI depend on? (Separate
  from the Emacs editor + keybinding model, which can follow.)

**Follow-ons (separable, lower priority).** Emacs-style line editor replacing
the display-only `InputEditor` (`editor.rs:46-64`, no state to migrate) — use
**`unicode-segmentation`** for grapheme/word boundaries (Rust `Intl.Segmenter`
equivalent), vendor kill-ring/undo as small internal structs; `KeyAction` enum
keybinding model with conflict-detecting insert (extend the existing
`keybindings.rs:85-128` `FromStr`); Kitty image lifecycle (chunked/imageId
reuse/delete — **non-trivial, own design spike**, unlike OSC 8 / OSC 11);
`ratatui-image` as idiomatic replacement for the hand-rolled `terminal_image.rs`
(fixes sixel-empty, kitty-non-PNG, no Ghostty/WezTerm/tmux detection);
crossterm Kitty-keyboard via `PushKeyboardEnhancementFlags` (crossterm has this
natively), OSC 8 + OSC 11 color queries need app-layer escape + reply parsing
(crossterm does **not** support OSC 11 natively — open issue #1037, input-contention
risk; weigh value before committing).

**Resolution (2026-07-11, grilling Q1–Q5; 5-agent research `wf_95882464-4dc`; 5-lens adversarial verification `wf_ffa4a66f-6e9` — core design holds across all lenses; material corrections C1–C5 folded).**
- **Perf — event-driven loop (Q1).** Replace the 50ms `event::poll` loop (`interactive.rs:341-689`) with a `tokio::select!` over FOUR arms: (1) crossterm `EventStream` (enable crossterm's off-by-default `event-stream` feature — verified: pulls only `futures-core`, already transitive, no runtime crate, runtime-agnostic, spawns its own OS thread, no feature conflict with ratatui-crossterm 0.1); (2) a `tokio::sync::watch<u64>` wake channel fed by the agent `subscribe` callback (`interactive.rs:136-319`) via `send_modify` (verified sync/non-blocking/`Send+Sync`/safe from the spawned agent task thread; `send_modify` chosen over `send_replace` as the lighter in-place counter bump; watch carries only a generation, no payload loss since TuiState still mutates under the existing mutex), the generation doubling as the dirty signal; (3) the pending `JoinHandle` awaited directly (replaces the `is_finished()` poll at `interactive.rs:358`; verified to compose with `cancel_token` + `harness.lock().await` — the mutex is awaited only in the post-completion branch, not held across arms); (4) **a streaming-redraw throttle** — `tokio::time::interval` armed ONLY while `app_state == Streaming`, coalescing token-wakes into ≤~N fps redraws, disarmed at idle so idle CPU stays at zero. No always-on animation interval (verified zero spinner/blink/elapsed/interval matches in opi-tui + interactive.rs).
- **C1 correction (load-bearing).** Without the streaming-redraw throttle, per-token `TextDelta`→`AgentEvent::MessageUpdate` wakes (`agent_loop.rs:620-639`) would drive `build_shell`+`s.messages.clone()` (full deep transcript clone, `interactive.rs:697-724`) at unbounded token-arrival rate — WORSE than today's 20fps batching. The throttle is mandatory, not optional. Refines the pre-verification "no interval" note: no ANIMATION interval, but YES a streaming-redraw throttle (pi `MIN_RENDER_INTERVAL_MS=16` analog; default rate ~16–32ms is a design-doc detail).
- **Dirty-gate.** Skip `build_shell`+`terminal::draw` when no input, no watch-wake, no task-completion, no throttle fire since last draw. **Load-bearing during streaming** (with the throttle, prevents O(token-rate) transcript clones), defensive at idle. Generation via `wrapping_add(1)` (u64 wrap is a non-issue, ~300y; compared for inequality only).
- **Premise confirmed (verified).** ratatui 0.30 cell-diffs at flush (`ratatui-core` `Buffer::diff` in `buffer.rs` + `Terminal::flush` in `buffers.rs` → backend draws only changed cells); opi does NOT send a full frame. Realign "full-frame redraw" wording imprecise; real cost is `build_shell` cloning the transcript + widget-tree walk ~20×/sec at idle. **Honest perf framing (C7):** opi does NOT adopt pi's changed-line-range batching + CSI ?2026h synchronized-output (a second render-output perf mechanism) — opi/ratatui does per-cell `MoveTo`+`Print` diff; T10 targets `build_shell`+widget-walk, not render-output batching. Do not overstate "matches pi render perf."
- **Event::Resize fix (C2).** Today's loop matches only `Event::Key` (`interactive.rs:403`); `Event::Resize`/`Paste`/`FocusGained` are silently dropped, papered over by ratatui's autoresize-on-draw every 50ms. EventStream delivers `Resize` natively → T10 **fixes the latent drop**; the EventStream arm handles `Resize` (+ `Paste`/`FocusGained`) explicitly.
- **Viewport::Inline/Fixed rejected** (opi is fullscreen alternate-screen). No animation tick.
- **OverlayStack — full pi parity (Q2), in opi-tui.** 9-anchor enum, `SizeValue` (cells | %), `OverlayHandle` (hide/setHidden/isHidden/focus/unfocus/isFocused), z-order via monotonic `focusOrder`, `retargetOverlayPreFocus` for nested chains, blocked-focus-restore state machine (inactive/eligible/blocked), `visible?(w,h)` callback, `nonCapturing`. **Rust-native mechanism (§3.2):** composite via ratatui `Clear` + render-widget-into-`Rect` per layer bottom-up by `focusOrder` (verified capability-equivalent to pi's opaque `compositeLineAt` line-splice — pi overlays are opaque, no alpha/partial-occlusion/scrolling lost; opi fullscreen makes positioning strictly simpler than pi's viewport-offset math). **Implementation hazard (C3):** `SizeValue`/margin-clamp arithmetic must use `i32` intermediates or `saturating_sub` — naive u16 port of pi's bounds-clamp underflows on negative effective height (panic in release). Introduces explicit `FocusTarget` model opi lacks today (focus is implicit fallthrough). `OverlayHandle` = typed generational index into `Vec<Layer>` (verified sound for `focus()`/`unfocus()` focusOrder bumps after pops — pi's object-identity can't cross to Rust, generational index is the correct translation). State on `TuiState` (replaces `Option<PickerOverlay>`), snapshotted into `Shell` per frame; top-of-stack key routing reuses the raw-`KeyCode` dispatch pattern from `handle_picker_key` (verified NOT dependent on T16's `KeyAction` model). Migrate the 3 pickers (model/session/branch) onto the stack — `PickerAction` semantics preserved (verified).
- **C5 honest labeling (full-parity is T13-substrate, not picker-driven).** Verified: the 3 migrating pickers exercise 1/9 anchors (Center), no nesting, no focus chains — `blocked-focus-restore` + `retargetOverlayPreFocus` + 8 extra anchors have ZERO current driver. Full parity rationalized by T13 (extension overlays); recorded by user choice (Q2 overrode the minimal recommendation). Labeled explicitly so it is not mistaken for picker-driven design. Fallback if T13 slips: reduce to minimal-stack + staged extension.
- **Palette NOT in T10 (Q3)** (pi has no palette — inline Editor autocomplete; overlay palette is divergence, no driver). **tui-popup NOT adopted (Q4)** (single-popup widget, experimental `PopupState`, redundant with opi's existing Clear+Rect; verified opi must build the stack/focus layer regardless). **Follow-ons OUT (Q5)** → graduate to T16/T17/T18 (verified specifiable; T16 is NOT a hidden dependency of T10).
- **Invariant preserved (verified).** opi-agent has zero ratatui/crossterm/Overlay/FocusTarget refs today; T10 introduces nothing in opi-agent (loop rewrite + OverlayStack + FocusTarget all in opi-coding-agent/opi-tui); subscribe seam (`Box<dyn Fn(&AgentEvent) + Send + Sync>`) unchanged; `agent_loop`/`AgentHooks`/`CompactionHooks`/`AgentLoopContext` untouched. (Precision note: the construction-ownership invariant is documented in MEMORY, not enforced in opi-agent's own source — `Agent::new` takes a caller-built `Box<dyn Provider>`; opi-agent has no provider-construction code to leak TUI into.)

**Scope boundary.** T10 delivers: event-driven render loop (`EventStream` + `watch` + `JoinHandle` + streaming-redraw throttle) + dirty-gate + `Event::Resize` fix, and the full-parity `OverlayStack` + `FocusTarget` model + 3-picker migration in opi-tui. NOT T10: palette, T16/T17/T18, the Phase 17 design doc, the §15 spec rewrite.

**Residuals / follow-ups.**
- **T13 partially unblocked (C4).** T10 provides the overlay-RENDERING substrate; T13 still owns the extension-facing `ExtensionUIContext` trait (`select`/`confirm`/`input`/`setWidget`/`custom` — exists NOWHERE in source today, only in docs; the `Extension` trait at `extension.rs:185-309` has no UI methods), its registration seam, and the dialog event loop. T13 owns the trait; cross-ref added to its ticket.
- Streaming-redraw throttle rate (default ~16–32ms, pi-parity, configurable) — design-doc detail.
- Full-parity overlay surface (9 anchors, blocked-focus-restore) has no current picker driver — T13 substrate; minimal-stack reduction is the fallback.
- §15 spec rewrite batched with the Phase 17 design doc → triggers phase4 + phase6 ledger re-sync (memory `spec-edit-breaks-phase4-ledger`).

#### T11 — Extension registry v2 (dynamic registration, event bus, RPC breadth) `grilling` · resolved 2026-07-11
**Question.** How do we (a) replace the startup-lock with **dynamic runtime
registration** — switch `ExtensionRegistry`'s `Arc<Vec>` to
`Arc<RwLock<Vec<…>>>` or `Arc<RwLock<IndexMap>>` (`RwLock` preferred, reads
dominate; `extension.rs:369-377`) — and decide revocation scope (per-call vs
registration-time-only); (b) grow the event surface — `AgentEvent` is **already
`#[non_exhaustive`** (`event.rs:14`), so the work is **adding variants + a shared
cross-extension `EventBus`** (distinct from the enum; has `Send`/`Sync`/lifetime
implications an enum growth does not — opi has only an `on_event` observer
dispatch today, `extension.rs:510-536`, no inter-extension bus); (c) grow RPC
breadth from 12 `SdkCommand` (`sdk.rs:58-129`) toward pi's **31** (corrected down from the ticket's "~35" estimate by verification)
`RpcCommand` variants — which subset is in-scope for Phase 17?
**Ownership.** Provider hooks + event bus live in `opi-agent` via trait seam;
overlay/ExtensionUIContext/renderers live in `opi-tui`/`opi-coding-agent`.

**Resolution (2026-07-11, grilling Q1–Q3; research + adversarial verification `wf_5e5e7cac-509`, 7 agents — both branches hold at high confidence).**
- **(a) Dynamic registration — DEFER.** Keep `extensions: Arc<Vec<Box<dyn Extension>>>` (`extension.rs:339`) + the implicit startup lock (`register()` fails `RegistryLocked` once `Arc::get_mut` returns `None` after `wrap_hooks`/`wrap_event_sink` clone the Arc). No Phase-17 driver: every production `register()` is pre-loop at startup (`runtime_packages.rs:25` → `adapter_extension.rs:886`); reads dominate the hottest paths (every tool call/event/turn); `CompositeHooks` holds a *snapshot* `Arc<Vec>` (`extension.rs:564`), so a mutable registry wouldn't reach already-wrapped hooks without re-reading per call. The `RegistryLocked` error + its 2 pinning tests (`tests/extensions.rs:433/452`) encode intentional correctness, not an ergonomic bug. Revocation-scope sub-decision moot. → fog (re-trigger: a concrete mid-session `load_extension`/hot-reload driver appears).
- **(b) Inter-extension EventBus — DEFER.** opi's flow is strictly one-way host→extensions (`AgentEventSink = Box<dyn Fn(AgentEvent)>` sink, `event.rs:11`; `dispatch_event` flat fan-out, `extension.rs:429-433`; `on_event` returns nothing, `:268`). Zero consumers — the only production extension (JSONL adapter) is a one-way fire-and-forget forwarder; no test shows two extensions coordinating (`tests/extensions.rs:471-529`); T12/T13/T14 are each served by their own mechanism (customType render dispatch / ExtensionUIContext request-response / AgentHooks seam). pi *does* have a bus (`event-bus.ts`, `pi.events.emit/on`) but the host never emits onto it — opi shipping without it is a **deliberate divergence**, not a pi gap. **Pinned fallback design** (decided, reuse when triggered): (1) NEW `ExtensionEvent` type, do NOT overload `AgentEvent`; (2) `Extension`-trait seam in opi-agent (consistent with `on_event`/`dispatch_event` ownership); (3) STATIC subscription at registration — deliberate divergence from pi's dynamic `pi.events.on`, justified by (a); (4) `serde_json::Value` payload (RPC-friendly); (5) fire-and-forget broadcast, registration-order dispatch, handler errors isolated + logged (mirror pi `createEventBus`). → fog (re-trigger: T13 lands AND shows an extension→host / extension↔extension signal need `extension_command` cannot serve).
- **(c) RPC breadth — principled tiered growth, NOT blanket pi parity.** Ticket's "~35" = verified **31** (`packages/coding-agent/src/modes/rpc/rpc-types.ts`, single discriminated union; no separate HostRpc). GAP = 19. **Principle (load-bearing):** a command becomes a first-class `SdkCommand` variant ONLY if it needs mid-turn delivery, streaming, or capability negotiation; otherwise route through `extension_command` — which is hard-blocked mid-turn by `ERR_AGENT_BUSY` (`rpc.rs:894-901`), single-JSON-response, first-Some-wins, no discovery, frozen at construction. **Commands ride with their implementing tickets** (T2/T8/T6 are design-resolved only — ZERO impl code today; "add now" would create the very stubs the current fully-wired dispatch avoids): `login`/`logout`/`login_status` + `get_available_models` → Phase 14 (T1/T2/T3); `move` (T8(d), typed `entry_id`) → Phase 16; `trust`/`set_trust` → Phase 15 (T6); session-tree read cluster (`get_tree`/`get_entries`/`get_entries_since`/`get_fork_messages`) → Phase 13/17. All handlers in `rpc.rs` (opi-coding-agent), calling their own store (preserves the opi-agent-must-not-own-auth invariant). **EXCLUDE:** `set_tools` (no driver — tool selection is startup-only `main.rs:336/452`, no runtime-mutation substrate); top-level `bash`/`abort_bash` (would bypass `agent_loop` tool-policy); image-gen/orchestrator/gallery/share-export-html clusters; a raw/generic escape hatch beyond `extension_command` (pi has none — mid-turn capability belongs to T13). **DEFER (Tier 3):** `cycle_model`/`cycle_thinking_level` (pi TUI keybinding-driven; low value in pure RPC), runtime-config toggles (`set_steering_mode`/`set_follow_up_mode`/`set_auto_retry`/`abort_retry`/`set_auto_compaction`). **Schema:** `SDK_SCHEMA_VERSION` stays **3** through Phase 17 additive growth (advertise-only at `rpc.rs:395`, no server enforcement; additive variants wire-safe — unknown→clean serde `Err`, pinned by `tests/rpc_jsonl.rs:145-149` + `tests/sdk_embedding.rs:240-244`); bump to 4 only on a true breaking change (variant removal / required-field add), not cargo-cult signaling. opi's `quit` + `trace` variants stay (no pi counterpart — documented deliberate divergence).

**Premise corrections (surfaced by verification).** Ticket's "~35" = verified **31**. T8 is "LLM compaction & branch-summary" (`:376`); `/move` is T8 sub-item (d) scoped INTO Phase 16 (`:426`), not "T8 move" standalone. `wrap_event_sink` (`extension.rs:525-536`, citation refreshed from the ticket's `:510-536`) is **LIVE embedder API** exercised by ~6 example tests (`mcp_adapter_example.rs`, `permission_gate_example.rs`, `todo_example.rs`, `plan_mode_example.rs`, `protected_paths_example.rs`, `sub_agent_example.rs`) — the harness production path bypasses it in favor of `agent.subscribe`+`dispatch_event` (`harness.rs:832`), but it MUST NOT be removed as dead code.

**Scope boundary.** T11 delivers the Phase-17 extension-surface **policy**: (a)/(b) deferred with pinned fallbacks + re-triggers; (c) the first-class-vs-escape-hatch principle, the tier mapping (commands ride with implementing tickets), the exclude list, and the schema-version discipline. The Phase 17 design doc (synthesizes T10+T11+T12+T13+T14), the §15 spec rewrite, and implementation (via `opi-implement`) are NOT T11.

**Residuals / follow-ups.**
- EventBus + dynamic-registration re-triggers → fog (graduated patches in Not yet specified).
- T13 owns the mid-turn-capable command channel via its separate `ExtensionUIContext` protocol (mirrors pi `RpcExtensionUIRequest`) — cross-ref added to T13.
- Update any downstream §15/design-doc text citing "~35" RPC variants → 31.
- §15 spec rewrite batched with the Phase 17 design doc → triggers phase4 + phase6 ledger re-sync (memory `spec-edit-breaks-phase4-ledger`).

#### T12 — Extension UI: inline renderers `grilling` · resolved 2026-07-11 (deferred)  · *(separated from T10 by verification; premise corrected)*
**Question.** How do we add **inline** message/entry renderers
(`registerMessageRenderer`/`registerEntryRenderer` parity) — a render-dispatch
hook in `opi-tui`'s `MessageList` keyed by `customType`, **no overlay stack
needed** (these render inline, not as z-axis layers)? This track is separable
from and does not block on T10's overlay stack.

**Resolution (2026-07-11, grilling Q1–Q2; research + adversarial verification `wf_9dd7bf3e-8a6`, 4 agents — verdict Corrects the ticket premise at high confidence).**
- **Scope — DEFER.** T12 is NOT "a render-dispatch hook in MessageList, separable from T10" (the ticket framing) — it is FOUR coupled substrate pieces: (i) `MessageRenderer` trait + ratatui-free primitive in opi-tui, (ii) an additive `custom` field on `opi_tui::Message`, (iii) a NEW `AgentEvent::CustomMessage` variant + an `interactive.rs` streaming-callback branch (the TUI transcript is built ONLY from streaming `AgentEvent`s — `interactive.rs:136-647`; resume pushes only `[session resumed]`, never replays), AND (iv) dispatch threaded into `MessageList`/`Shell` at frame build. With ZERO runtime producers this is infrastructure for an empty registry — the same no-driver condition that deferred T11(a)/(b). → fog (re-trigger: a concrete extension needs structured transcript rendering beyond a formatted system-line string).
- **Entry-renderer half — DROPPED.** `SessionEntry` has 9 variants and NO Custom/custom_message (`session.rs:197-208`; module doc `:33-38` "custom_message is deferred"). pi's `registerEntryRenderer` has no opi render target. A TUI-only custom-entry substrate would be a separate ticket, not part of T12.
- **Architecture (SURVIVES, crate-invariant-correct).** `MessageRenderer`/`EntryRenderer` traits live in **opi-tui**, registered via **opi-coding-agent** (the mediator); `opi-agent`'s `Extension` trait is untouched (UI-free). Mirrors the T5/T8/T11 seam-placement principle (a trait lives where its runtime invokes it — rendering is opi-tui/opi-coding-agent, never opi-agent).
- **Pinned fallback design (Q2, reuse when triggered):**
  1. New PUBLIC ratatui-free primitive IN opi-tui: `pub enum RenderedContent { Lines(Vec<RenderedLine>), Empty }`, `RenderedLine { text: String, fg: Option<RenderColor>, bold: bool }`; `RenderColor` reuses the existing `parse_color`/`THEME_TOKENS` surface (`lib.rs:34`). opi-tui converts `RenderedContent → ratatui::Line` internally inside `MessageList::render`. Extensions depend on opi-tui only — NEVER ratatui.
  2. REJECT `Box<dyn Widget>` (ratatui `Widget` not cleanly object-safe; leaks ratatui into the trait sig) and `Vec<ratatui::text::Line>` (forces ratatui dep on every extension impl).
  3. FLAT lines only — no Box-like nesting (pi `Box`/`Text` composition has no clean ratatui equivalent; flat lines cover dispatch+data parity; richer composition = later additive variant).
  4. Width hint passed at call time — deliberate divergence from pi (pi gets width later at `Component.render`; opi-tui recomposes per frame + ratatui layout is area-driven). Document on ship.
  5. Dispatch key = `AgentMessage::Custom`'s `kind` field (snake_case — NOT pi's `customType`; the discriminator already exists at `message.rs:20,44-48`). Unknown `kind` → default render, no error.
  6. Registration is startup-time (consistent with T11(a) defer of dynamic registration).

**Premise corrections (surfaced by verification).** (a) Ticket's "keyed by `customType`" — opi's discriminator is `AgentMessage::Custom(CustomAgentMessage { kind, .. })` (`message.rs:20,44-48`), and it has ZERO runtime producers (only tests construct it; `harness.rs:2251` drops it `_ => {}`; `compaction.rs:257` stringifies it). (b) Ticket's "entry renderers" half — no `SessionEntry::Custom` exists. (c) Ticket's "a render-dispatch hook … separable from T10" — not separable: needs a new `AgentEvent::CustomMessage` + `interactive.rs` conversion because the TUI transcript is streaming-forward-only (no history replay). (d) opi-tui does NOT re-export ratatui `Line`/`Color` (`lib.rs:34`), so the primitive must be opi-tui-native.

**Bookkeeping note (not a correction).** Verifier flagged "T10 not in code" as CRITICAL — that is a misread of the wayfinder model: *resolved* = design settled; implementation is downstream `opi-implement` and explicitly out of scope for every ticket T1-T12. T10's loop rewrite is orthogonal to T12's inline dispatch regardless. No action.

**Scope boundary.** T12 delivers the Phase-17 inline-renderer **policy**: deferred with a pinned crate-correct architecture + ratatui-free primitive + re-trigger, and the entry-renderer half dropped. The Phase 17 design doc (synthesizes T10+T11+T12+T13+T14), the §15 spec rewrite, and implementation (via `opi-implement`) are NOT T12.

**Residuals / follow-ups.**
- Inline-renderer re-trigger → fog (graduated patch in Not yet specified, primitive pinned so it isn't re-litigated).
- The 4-piece producer path (if a future ship-now decision) is the real scope: opi-tui trait+primitive, `Message.custom` field, `AgentEvent::CustomMessage`+`interactive.rs` conversion, `MessageList`/`Shell` dispatch threading.
- opi's streaming-forward-only transcript (no resume replay) is intentional and out-of-scope for T12 — custom rendering applies to live-streamed messages only; document on ship.
- §15 spec rewrite batched with the Phase 17 design doc → triggers phase4 + phase6 ledger re-sync (memory `spec-edit-breaks-phase4-ledger`).

#### T13 — Extension UI: overlay/dialog surface `grilling` · resolved 2026-07-11 (deferred with pinned design) (T10 rendering substrate resolved 2026-07-11)
**Question.** How do we expose the `ExtensionUIContext` overlay/dialog surface
(`select`/`confirm`/`input`/`setWidget`/`custom` overlays — pi
`types.ts:126-277`) on top of T10's `OverlayStack`, with a Rust-native trait seam
so the extension-facing API lives in `opi-coding-agent`/`opi-tui`, not
`opi-agent`? **T10 (resolved) provides the overlay-RENDERING substrate only (C4 in
T10 resolution): T13 still owns the `ExtensionUIContext` trait itself**
(`select`/`confirm`/`input`/`setWidget`/`custom` — exists NOWHERE in source today,
only in docs; the `Extension` trait at `extension.rs:185-309` has no UI methods),
its registration seam, and the dialog event loop. T10 is necessary-but-not-sufficient.

**T11 cross-ref (mid-turn command channel).** `extension_command` is hard-blocked mid-turn by `ERR_AGENT_BUSY` (`rpc.rs:894-901`) — any overlay/dialog command that must inject while the agent streams CANNOT use it. Per the T11(c) resolution, T13 owns the mid-turn-capable command channel via this ticket's separate `ExtensionUIContext` protocol (mirrors pi `RpcExtensionUIRequest`), NOT a T11 `SdkCommand` variant.

**Resolution (2026-07-11, grilling Q1–Q3; research + adversarial verification `wf_22070225-2fe`, 5 agents — verdict recommendation_holds=FALSE at high confidence: crate-placement core SURVIVES, three design flaws corrected, driver condition identical to T11/T12).**
- **Scope — DEFER (mirror T11/T12).** Zero in-tree drivers: the JSONL adapter is fire-and-forget; T6 `AppState::AwaitingTrust` and T2 `LoginPresenter` are DESIGN-ONLY (no `.rs` code — `AppState` at `lib.rs:138-142` has only `{Idle,Thinking,Streaming,ToolExecuting}`, no AwaitingTrust; grep finds no LoginPresenter/CredentialNeeded/ExtensionUiContext). T13 generalizes the pause-and-ask substrate T6/T2 will establish — building the generalization BEFORE the concrete cases risks re-litigating the seam once they land. Let T6/T2 land first with ad-hoc pause-and-ask, then T13 generalizes. → fog (re-trigger: T6 OR T2 lands pause-and-ask in code AND needs generalizing; OR a concrete extension needs an overlay).
- **Trait location — opi-coding-agent (VERIFIED).** `agent_loop` calls `tool.execute` (`agent_loop.rs:803`), never a UI method; `Extension` (`extension.rs:185-326`) has zero UI methods. Invoked by opi-coding-agent code (tools/extensions/hooks), not opi-agent's loop → lives in opi-coding-agent per the T5/T8/T11/T12 deciding principle. NOT opi-tui (carries async request/response + RPC semantics, not just rendering).
- **Pinned concurrency design (Q2, reuse when triggered):**
  1. INJECTION, not widening: inject `Arc<dyn ExtensionUiContext>` at `build_tools` (`harness.rs:1896`) — extension/hook-only. Do NOT widen `Tool::execute` (`tool.rs:21-27` has 4 args; widening breaks every tool + the trait). Forecloses a future "ask-user" built-in TOOL until `Tool::execute` is widened — acceptable (T6 trust hook + T2 OAuth are extension/hook-based). Extensions receive it via a default-no-op `set_ui_context(&self, ctx)` mirroring `set_trace_collector` (`extension.rs:325`).
  2. TWO transport modes (not one bridge): **INTERACTIVE** — tool future writes `PendingUiRequest{id, req, tx: oneshot::Sender}` into `Arc<Mutex<TuiState>>`; the 50ms poll loop or T10's `watch<u64>` picks it up, renders an overlay reading ONLY `TuiState` (lock-free of harness, `interactive.rs:352`), user choice fulfills the pre-stored oneshot → tool resumes; NO schema surface. `subscribe` is SYNC (`Box<dyn Fn(&AgentEvent)>`), so the oneshot MUST be pre-stored (two-part handshake). **RPC** — emit `AgentEvent::UiRequest{id,kind,data}` (additive, `#[non_exhaustive]`, schema stays 3 per T11(c)) + add `SdkCommand::ui_response{id,data}` as a first-class mid-run command (`extension_command` is hard-blocked by `ERR_AGENT_BUSY`, `rpc.rs:894`); pending-registry `Arc<Mutex<HashMap<RequestId, oneshot::Sender>>>` on `RpcRunner`.
  3. DEADLOCK GUARD (achievable, verified): overlay render reads `Arc<Mutex<TuiState>>` (lock-free), response rides the oneshot — both bypass the `harness.lock().await` held across `prompt().await` at `interactive.rs:655-658`. Document on ship.
  4. CANCELLATION: `CancellationToken` drop → suspended tool future dropped → oneshot `Receiver` dropped → `Sender::send` returns `Err` (benign). User-ESC → resolve oneshot to default (None/false, pi parity), fail ONLY the tool (T6 dismiss-only precedent).
  5. PARALLEL-BATCH policy: FIFO serialize simultaneous `ui.select()` calls (pi is single-overlay).
  6. Non-interactive default: `ToolError::ExecutionFailed` / `is_error` `ToolResult` (`tool.rs:86`, agent_loop already converts) — the real failure primitive today; "fail-the-turn like T2 CredentialNeeded" is aspirational. Deliberate divergence, document on ship.
- **Pinned split-trait scope (Q3, reuse when triggered):** SPLIT, not monolithic. **`BlockingDialogUi`** = `select`/`confirm`/`input`/`notify` — transient modal on the existing `PickerOverlay` pattern (`interactive.rs:47-100`), NO OverlayStack dependency; the T6/T2 set (pi's own `ProjectTrustContext.ui` is the same `Pick<...,'select'|'confirm'|'input'|'notify'>` at `types.ts:517`). **`PersistentOverlayUi`** = `setWidget`/`custom` — behind T10's OverlayStack (design-only today; `custom` supplies a T12 `RenderedContent` renderer + gets an `OverlayHandle` via `on_handle`). **`editor` deferred** to T16 (multiline editor widget). RPC impl fully satisfies `BlockingDialogUi`; no-ops `PersistentOverlayUi`/`custom` (TUI-only, mirrors pi's `rpc-mode.ts`). `OverlayHandle` exposed as an opaque `ExtensionOverlayHandle` owned by opi-coding-agent (ratatui-free invariant); `setWidget` identity = pi's caller-chosen string key (generational handle enters via `custom`'s `on_handle`).

**Premise corrections (surfaced by verification).** (a) "tools receive context via build_tools" — `Tool::execute` has NO context arg (`tool.rs:21-27`); it's constructor injection, not a 5th arg (`UpdateCallback` is dead plumbing, passed `None` at `agent_loop.rs:803`). (b) Single `AgentEvent::UiRequest + SdkCommand::ui_response` bridge — it's TWO: interactive needs NO schema change (TuiState polling suffices), only RPC needs the wire pair. (c) `subscribe` is SYNC — cannot carry a oneshot back; the response rides a pre-registered oneshot. (d) T6/T2 are design-only, not shipped — weakens (does not flip) the defer.

**Bookkeeping note.** Verifier's "T10 not in code" reappears — same misread as T12 (resolved = design settled; implementation is downstream `opi-implement`). T10's OverlayStack is the substrate `PersistentOverlayUi` waits on; recorded as a dependency, no action.

**Scope boundary.** T13 delivers the Phase-17 overlay/dialog **policy**: deferred with a pinned crate-correct trait location + two-transport concurrency design + deadlock guard + split-trait scope, recorded so re-trigger does not re-litigate. The Phase 17 design doc (synthesizes T10+T11+T12+T13+T14), the §15 spec rewrite, and implementation (via `opi-implement`) are NOT T13.

**Residuals / follow-ups.**
- ExtensionUIContext re-trigger → fog (graduated patch in Not yet specified, design pinned).
- When triggered, ship `BlockingDialogUi` first (PickerOverlay-based); `PersistentOverlayUi` waits on T10 OverlayStack in code.
- §15 spec rewrite batched with the Phase 17 design doc → triggers phase4 + phase6 ledger re-sync (memory `spec-edit-breaks-phase4-ledger`).

#### T14 — Provider request/response hooks `grilling` · resolved 2026-07-11 (deferred with pinned design)  · *(separated, co-designed with T3d; premise corrected)*
**Question.** How do we add `before_provider_request`/`before_provider_headers`/
`after_provider_response` to `AgentHooks` (`hooks.rs:59-106` has no provider hook
today), mutating opi-ai `Request`/response directly? `opi-agent` already imports
`opi-ai` types so this adds no new dependency — but frame it as a **seam, not a
provider call** (the construction-ownership invariant). Co-design the
`onPayload`/`onResponse` hook shape (T3d) here. **Does not block on T3
`StreamOptions` scalars** — `Request` already exists and can carry the mutation.

**Resolution (2026-07-11, grilling Q1–Q2; research + adversarial verification `wf_9f09c0ec-3ec`, 5 agents — verdict DEFER holds at high confidence; hook count corrected 3→1).**
- **Scope — DEFER (mirror T11/T12/T13).** opi-spec.md:1491 ALREADY classifies provider hooks as a Future Candidate deferred "until the provider seam and trace/redaction semantics stabilize"; those prerequisites are now met IN DESIGN (Phase 7 redaction + Phase 12 correctness + T3 Request enrichment), so the spec is effectively waiting on a CONSUMER, not prerequisites. Zero in-tree consumers (grep before_provider/after_provider = no hits; example extensions use `on_before_tool_call`). Every candidate driver is covered: auth→T2 `AuthSource` (D8 rejected per-call override), headers→T3a `extra_headers` (design-only, NOT code-landed — strongest driver BLOCKED on T3a), observability→`MessageEnd` + `Usage` + trace envelope (`trace.rs:71-74`), redaction→Phase 7 at the trace/event layer. → fog (re-trigger: a concrete extension/embedder needs dynamic per-turn model/message/header mutation — compliance rewrite, A/B prompt, DLP gate — that `AgentEvent` observation + T3a static headers + T2 `AuthSource` cannot serve; wait for T3a code-landing before re-evaluating the headers driver).
- **Hook count — 1, not 3 or 2 (ticket framing CORRECTED).** The ticket's 3-hook framing (`before_provider_request` / `before_provider_headers` / `after_provider_response`) is cargo-cult: pi split `before_provider_headers` because pi's `before_provider_request` REPLACES the payload by return value (headers unreachable); opi mutates `&mut Request` IN PLACE (`agent_loop.rs:90-101`), so header injection is a field write once T3a lands. `after_provider_response` dropped — `MessageEnd` (`event.rs:41`, fired at `agent_loop.rs:154-159`) + `on_event` already cover observation; there is NO aggregated Response struct (terminal `Done{message}` yields `AssistantMessage` at `:660`).
- **Pinned design (Q2, reuse when triggered):** ONE hook — `fn before_provider_request(&self, req: &mut opi_ai::provider::Request) -> Pin<Box<dyn Future<Output = Result<(), AgentError>> + Send>>` with default `Ok(())`, on `AgentHooks` (opi-agent; additive, non-breaking — mirrors `transform_context` at `hooks.rs:67-73`). Fire BEFORE `validate_request_capabilities` (`:102`) so model/message mutations re-validate free. `provider.stream(request)` takes it by value at `:118`, so mutations between `:101` and `:118` reach the wire. Chain base-then-extensions in `CompositeHooks` (`extension.rs:562-657`). opi-agent already imports `opi_ai::provider::Request` (`agent_loop.rs:8`) — no new dependency; seam, not a provider call (invariant preserved). T3d streaming (`onPayload`/`onResponse`) stays its OWN fog entry — do not bundle.
- **Two footguns (pin in design doc, resolve at trigger):** (1) `request.cancel` is the SAME `CancellationToken` used in the `tokio::select!` cancellation race (`agent_loop.rs:128-140`) — the hook MUST NOT mutate it; freeze (pass `&mut` to a Request-minus-cancel wrapper) or document loudly. (2) Abort-via-`Err(AgentError)` classification: opi tool-hook-style `Deny` (opi parity, pi divergence) vs throw-to-abort only (pi parity) — decide at trigger time. Optional: if process-jsonl adapters need it, add `on_before_provider_request` to the `Extension` trait (a SECOND trait-method addition composed in `CompositeHooks`) — only if an adapter driver appears.
- **Spec amendment (batched):** opi-spec.md:1491 gets a one-line amendment recording prerequisites-met-in-design + consumer-pending, BATCHED with the §15 rewrite (both edit opi-spec.md → one phase4+phase6 ledger re-sync, memory `spec-edit-breaks-phase4-ledger`).

**Bookkeeping.** `request.metadata` is set to `None` at `agent_loop.rs:99` and read by NO provider today (only model/system/messages/tools/max_tokens/temperature/thinking/stop_sequences/cancel reach the wire) — a before-provider hook mutating metadata would be a no-op until providers consume it. Recorded.

**Scope boundary.** T14 delivers the Phase-17 provider-hook **policy**: deferred with a pinned 1-hook design + footguns + re-trigger, hook count corrected 3→1. The Phase 17 design doc, the §15 spec rewrite (+ :1491 amendment), and implementation (via `opi-implement`) are NOT T14.

### Cross-phase / small

#### T15 — stdin/stdout TTY auto print-mode `task` · resolved 2026-07-11 (accepted) · *(promoted by verification, borderline)*
**Question.** Should opi auto-switch interactive → print when stdin/stdout is
non-TTY and read piped stdin as the prompt (pi `resolveAppMode`, `main.ts:107`),
via `std::io::IsTerminal` (Rust 1.70+)? Today `echo prompt | opi` blocks waiting
for interactive input. Small, bounded, user-ergonomics. **Accept or reject?**
(If accepted, place as a small task in Phase 14 CLI/runtime or standalone.)

**Resolution (2026-07-11, grilling; grounded in `main.rs:138-181` — ACCEPTED).**
- **ACCEPT.** opi auto-switches interactive→print when **stdin is non-TTY** AND **not `--rpc`** AND **no positional prompt** — reads piped stdin as the prompt, routes to non-interactive. A minimal addition to the `else`-interactive branch of the 3-way mode split (`main.rs:138-181`). First non-deferred ticket in the roadmap: T11–T14 deferred under no-driver; T15 has a concrete driver (piping into opi hangs today).
- **Trigger — stdin non-TTY only.** `std::io::stdin().is_terminal()` (stable since Rust 1.70; opi is 1.97+ → no new dep, no `atty` crate). stdout-TTY (`opi | cat`) is OUT of scope — pi's `resolveAppMode` checks both, but stdout-auto-switch is more invasive and a separate bug; stdin-only covers the `echo prompt | opi` case.
- **`--rpc` safe:** checked FIRST (`if cli.rpc`), so the TTY auto-switch in the non-rpc path never touches RPC's stdin-JSONL reader. No conflict.
- **Positional-wins:** if a positional prompt arg is present, use it and do NOT read stdin (avoids ambiguity). `echo p | opi --json` fills `--json`'s otherwise-empty prompt slot (small improvement, consistent).
- **Empty piped stdin** → existing `"no prompt provided"` error (`main.rs:315`); correct, no change.
- **Placement:** standalone CLI fix in `main.rs` (runtime/ergonomics), NOT bound to a phase (Phase 14 = Provider/Auth; unrelated). Small and concrete enough to land directly via a fix commit or `opi-implement` — no phase design doc required.

**Scope boundary.** T15 is a `task` ticket: the decision (accept) + the small design above is the deliverable. Implementation is a direct `main.rs` change (downstream, not a phase doc). The §15 spec rewrite is unaffected.

#### T16 — Emacs-style line editor + KeyAction keybinding model `grilling`+`prototype` · resolved 2026-07-11 (Tier 1 ACCEPTED / Tier 2 DEFERRED)
**Question.** How do we replace the display-only `InputEditor` (`editor.rs:14-64`,
a bordered Paragraph of placeholder/text with no cursor/editing) with a Rust-native
Emacs-style line editor (kill-ring, undo-stack, word-navigation, paste-marker grapheme
handling), AND extend the 3-field `Keybindings` (`keybindings.rs:38-42`
submit/abort/new_line, 5-key `Key` enum) into a `KeyAction` model with
conflict-detecting insert (pi `TUI_KEYBINDINGS` ~31 actions across editor/input/select
groups, `keybindings.ts:54`)? Use `unicode-segmentation` for grapheme/word boundaries
(Rust `Intl.Segmenter` equivalent); vendor kill-ring/undo as small internal structs;
extend the existing `keybindings.rs:85-128` `FromStr`. **Verified NOT a dependency of
T10** (overlay routing uses raw `KeyCode` dispatch, not `KeyAction`); targets the MAIN
prompt `InputEditor`, distinct from overlay filter input. Phase 17; promotes the prior
"Emacs editor" fog patch.

**Resolution (2026-07-11, grilling Q1; research + adversarial verification `wf_3719b0e6-667`, 5 agents — SPLIT-TIER; crux fact VERIFIED, ticket's display-only claim is CORRECT this time).**
- **CRUX (verified by direct read).** The ticket's "display-only, no editing" claim is ACCURATE (unlike T10's "full-frame redraw" misread): `InputEditor` (`editor.rs:14-64`) is a render widget — `Line::from(self.text.as_str())` at `:60`. The ENTIRE editing surface is `interactive.rs:677-685`: `Char(c) => input_text.push(c)` (append-at-END) + `Backspace => input_text.pop()` (delete LAST char only). No cursor index, no Left/Right/Home/End/Delete, no insert-in-middle. `show_cursor()` at `:336` is teardown-only — NO cursor renders during the loop. Fixing a typo at position 3 of a 100-char prompt requires backspacing 97 chars. A real core-usability gap in the DEFAULT interactive mode (like T15), not speculative parity.
- **SPLIT-TIER verdict.** Tier 1 (minimal cursor editor) = SHIP as core gap. Tier 2 (full Emacs + KeyAction enum) = DEFER as speculative pi-parity (opi's interactive surface is ~8 keystrokes; conflict-detecting KeyAction pays at ~12-15+; no in-tree consumer needs ~29 actions).
- **Tier 1 — ACCEPTED scope (ship):**
  1. `cursor: usize` on `TuiState` alongside `input_text` (do NOT revive `InputEditor` as a stateful widget — defer that rewiring to Tier 2); `render.rs:135` passes cursor to `InputEditor::new(text, cursor)`.
  2. Cursor-aware ops replacing the 3-arm match: insert-at-cursor, backspace-at-cursor, Left/Right/Home/End/Delete as raw `KeyCode` arms alongside the existing Char/Backspace fallback (`interactive.rs:677-685`).
  3. Extend the 3-field `Keybindings` struct with `Option<KeyCombo>` newcomers (cursor_left/right/home/end/delete) — do NOT adopt a `KeyAction` enum (over-engineering at ~8-10 bindings; `from_config_map:190-202` has no conflict possible at 3 named slots).
  4. Visible block cursor via ratatui `Style::reversed` on the cursor cell (widget-local; the teardown-only `show_cursor` renders nothing during the loop — a rendered cursor is strictly better; do NOT use `SetCursorPosition`).
  5. Fix the multi-line rendering degeneracy at `editor.rs:60` (`Line::from(text)` → `Text` of multiple Lines split on `\n`) — REQUIRED for a row+col cursor, not optional (Alt+Enter's pushed `\n` currently doesn't visually break).
  6. Promote `unicode-segmentation` to `[workspace.dependencies]` for grapheme-correct cursor (avoids a known desync on emoji/combining chars; already transitive per `Cargo.lock`, no new download; `unicode-width` already direct at `Cargo.toml:54` is NOT sufficient — display width ≠ grapheme boundaries).
- **Tier 2 — DEFERRED (full Emacs + KeyAction).** kill-ring, yank/yank-pop, undo-stack, word-navigation (M-F/M-B/M-D), paste-marker atomic grapheme segments, char-jump, AND the ~29-action `KeyAction` enum with conflict-detecting insert (pi `TUI_KEYBINDINGS`). No driver beyond pi-parity. → fog (re-trigger: (a) concrete power-user request for kill-ring/yank/undo/word-nav; (b) binding count crosses ~12-15 [auto-triggers KeyAction migration]; (c) concrete accessibility driver).
- **Decoupled from T10** (overlay routing at `interactive.rs:861` uses raw `KeyCode` via `handle_picker_key`, not KeyAction). Targets the MAIN prompt path, not overlay filter input.

**Note.** Tier-1 scope recorded from the verified brief without a separate grilling round (the user paused the map at this ticket); the brief is adversarially verified at high confidence, but scope may be adjusted at implementation time.

**Scope boundary.** T16 Tier 1 is a concrete implementable fix (downstream `opi-implement`, no phase design doc needed — like T15). Tier 2 is fog. The §15 spec rewrite is unaffected.

#### T17 — Terminal image protocol overhaul `research`+`grilling` · open
**Question.** How do we replace the hand-rolled `terminal_image.rs` (Kitty PNG-only
single-chunk, no `imageId`/chunking/delete; Sixel stub returns empty; detection checks
only 2 env values) with the idiomatic `ratatui-image` crate AND implement full Kitty
chunked lifecycle (4096-byte chunks, `imageId` reuse, `allocateImageId`,
`deleteKittyImage`/`deleteAllKittyImages`) + broad terminal detection
(Ghostty/WezTerm/tmux/screen/etc.)? Verified against pi `terminal-image.ts:65` (chunked
transmission, `imageId` reuse, delete, 11+ terminal detection). Own design spike
(non-trivial). Fixes sixel-empty, kitty-non-PNG, and no-Ghostty/WezTerm/tmux detection.
Phase 17.

#### T18 — Terminal capability negotiation (Kitty-keyboard + OSC 8 + OSC 11) `grilling` · open
**Question.** How do we add (a) crossterm Kitty-keyboard protocol via
`PushKeyboardEnhancementFlags` (crossterm-native, flags 7, fragment-timeout,
release/repeat events) replacing opi's `enable_raw_mode()`-only input; (b) OSC 8
hyperlink emission (`\x1b]8;;url\x1b\\ text \x1b]8;;\x1b\\`) with per-terminal gating;
(c) OSC 11 background-color query (`\x1b]11;?\x07`) + CSI ?996n color-scheme query for
theme detection? Verified against pi (`terminal.ts:14` Kitty-keyboard flags 7;
`terminal-image.ts:478` OSC 8; `tui.ts:1665` OSC 11/996). **crossterm does NOT support
OSC 11 natively** (open issue #1037) — needs app-layer escape + reply parsing
(input-contention risk; weigh value before committing). Phase 17.

## Not yet specified (fog)

- **Dynamic extension registration.** Graduated from T11(a) (resolved 2026-07-11
  as DEFER). No Phase-17 driver — every production `register()` is pre-loop at
  startup. Re-trigger: a concrete mid-session `load_extension` / package
  hot-reload driver appears. If triggered, switch `ExtensionRegistry.extensions`
  to `Arc<RwLock<…>>` AND thread it through `CompositeHooks` (which holds a
  snapshot `Arc<Vec>` today, `extension.rs:564`); then resolve the
  revocation-scope sub-decision (per-call re-read vs registration-time-only).
- **Inter-extension EventBus.** Graduated from T11(b) (resolved 2026-07-11 as
  DEFER). opi's event flow is one-way host→extensions with zero consumers; pi
  has a bus (`pi.events.emit/on`) opi deliberately doesn't ship. Pinned fallback
  design (reuse when triggered): new `ExtensionEvent` type (not `AgentEvent`),
  `Extension`-trait seam in opi-agent, STATIC subscription at registration
  (divergence from pi's dynamic `.on`, justified by T11(a)), `serde_json::Value`
  payload, fire-and-forget broadcast, handler errors isolated + logged. Re-trigger:
  T13 lands AND shows an extension→host / extension↔extension signal need that
  `extension_command` cannot serve.
- **Inline message/entry renderers (pi `registerMessageRenderer`/`registerEntryRenderer`).** Graduated from T12 (resolved 2026-07-11 as DEFER). Zero runtime producers of `AgentMessage::Custom` today; shipping needs 4 coupled pieces (opi-tui trait+primitive, `Message.custom` field, `AgentEvent::CustomMessage`+`interactive.rs` streaming conversion, `MessageList`/`Shell` dispatch). Pinned crate-correct architecture + ratatui-free primitive (`RenderedContent`/`RenderedLine`/`RenderColor` in opi-tui) recorded in ticket T12 — reuse without re-litigating. Entry-renderer half void (no `SessionEntry::Custom`). Re-trigger: a concrete extension needs structured transcript rendering beyond a formatted system-line string.
- **ExtensionUIContext overlay/dialog surface (pi select/confirm/input/setWidget/custom).** Graduated from T13 (resolved 2026-07-11 as DEFER). Zero in-tree drivers; T6/T2 pause-and-ask is design-only. Pinned crate-correct design recorded in ticket T13 — trait in opi-coding-agent, `build_tools` injection (no `Tool::execute` widening), two-transport bridge (interactive `TuiState`+oneshot / RPC `AgentEvent::UiRequest`+`SdkCommand::ui_response`), deadlock guard, split trait (`BlockingDialogUi` on `PickerOverlay` + `PersistentOverlayUi` behind T10 OverlayStack). Re-trigger: T6 OR T2 lands pause-and-ask in code AND needs generalizing; OR a concrete extension needs an overlay. When triggered, ship `BlockingDialogUi` first.
- **Provider request/response hooks (turn-level, on AgentHooks).** Graduated from T14 (resolved 2026-07-11 as DEFER). opi-spec.md:1491 already defers this; prereqs design-met, consumer pending. Pinned 1-hook design recorded in ticket T14 — `before_provider_request(&mut opi_ai::Request)` on AgentHooks, fire before `validate_request_capabilities` (`:102`), chain in `CompositeHooks`, `request.cancel` frozen (same token as the `tokio::select!` race `:128-140`), abort semantics at trigger. NOT the 3-hook pi split (opi mutates in place, no replace-by-return constraint); `after_provider_response` dropped (`MessageEnd` covers). Re-trigger: a concrete consumer needs per-turn provider-traffic mutation beyond T2 `AuthSource` + T3a static headers + `AgentEvent` observation. Distinct from the T3d streaming `onPayload`/`onResponse` fog entry.
- **Full Emacs line editor + KeyAction keybinding model.** Graduated from T16 Tier 2 (resolved 2026-07-11 as DEFER). No driver beyond pi-parity (opi interactive surface ~8 keystrokes; conflict-detecting KeyAction pays at ~12-15+). Pinned design: kill-ring/yank/yank-pop/undo-stack/word-nav/paste-marker grapheme segments vendored as small internal structs (opi-tui renders its own widget — do NOT adopt rustyline/reedline); ~29-action KeyAction enum with conflict-detecting insert (pi `TUI_KEYBINDINGS` model); revive `InputEditor` as a stateful widget then. Re-trigger: (a) concrete power-user request, (b) binding count crosses ~12-15, (c) accessibility driver. Distinct from T16 Tier 1 (minimal cursor editor, shipped).

- **Multi-API provider (api-map) internals.** Larger architectural change to the
  `Provider` trait/dispatch (one provider exposing multiple wire APIs per model,
  pi `models.ts:324`). Coarse; sharpen after T3 settles the `Request` surface and
  T2 settles per-request auth routing.
- **Image-generation (cluster I) internals.** Depends on T2 auth seam (verified:
  pi `generateImages` calls `resolveProviderAuth`, `images-models.ts:187`).
  Sharpen after Phase 14.
- **Extension-UI dialog protocol details.** T10 overlay-stack API resolved
  (2026-07-11); now depends on T13 surface (T13 still owns the `ExtensionUIContext`
  trait — see C4 in T10 resolution).
- **Orchestrator process model** (radius / rpc-process / IPC-serve / supervisor).
  New substrate; sharpen after RPC (`SdkCommand`) surface stabilizes (T11).
- **Overlay-based command palette.** Graduated from T10 (palette deferred — Q3).
  pi has no palette (inline Editor autocomplete); an overlay palette would be
  divergence. The full-parity OverlayStack (T10) supports one. Sharpen if a palette
  driver appears.
- **Animation tick / spinner cadence.** Graduated from T10 (no animated UI today —
  verified). Distinct from the in-scope streaming-redraw throttle. Sharpen if an
  animated-UI driver appears (spinner, blinking cursor, elapsed timer).
- **Viewport::Inline/Fixed for a print/inline render mode.** Graduated from T10
  (rejected for fullscreen interactive — Q1). Sharpen if opi adds a non-fullscreen
  render mode.
- **Per-call credential override (pi `ApiStreamOptions`).** Deferred from T2 (D8)
  — per-call apiKey/headers/env override on top of the resolved `AuthSource`; no
  multi-tenant use case yet. Sharpen when an extension/multi-tenant driver appears.
- **onPayload/onResponse streaming hooks (T3 3d).** Deferred from T3 — per-chunk
  streaming interception (logging/metering/proxying); needs a Rust-native design
  (trait object on a `RequestContext` vs `mpsc` channel vs `AgentHooks`
  extension) since closures can't live on a serde-derived `Request`. Distinct
  from T14's turn-level provider hooks. Sharpen when a streaming-observation
  driver appears.
- **Per-adapter sandbox-capability declaration schema.** Graduated from T4's
  adapter-strict-confinement deferral (D7). Bash is the sole Phase-15 `strict`
  surface because adapters need a manifest-declared capability mechanism
  (`[adapter] allow_network`/`allow_paths`) to confine without breaking
  legitimate packages. The sandbox *mechanism* (T4) is generic — wiring it to
  adapters is a config step once declarations exist. Sharpen after T6 (project-trust gate) resolves — the T5 Operations seam
  landed 2026-07-11.
- **Nav-tool remote-delegatable Operations (grep/find/ls/glob).** Graduated from
  T5's nav-tool deferral (D3). The `ignore`-crate walker (`nav_walk_builder`,
  `tool/mod.rs:199`) drives a local-FS traversal and can't be cleanly redirected
  to a remote backend; a remote-aware walker (delegated `readdir`/`stat`/
  `read_file`) is a larger redesign that would sacrifice gitignore semantics
  unless re-implemented remotely. pi's nav Operations are narrow and never
  overridden in practice. Sharpen when a concrete remote-nav use case appears.
- **Image-input model-capability gating + tool-level skip.** Graduated from
  T9 (Q7). `ModelCapabilities` (T3) lacks `supports_image_input`; today 4
  Mistral models (`supports_images=false`, `mistral.rs:37-64`) silently
  degrade read images to the companion-Text label. Add the capability + skip
  image decode/send for text-only models when a driver appears (broader
  text-only catalog, or per-model cost control).
- **Provider-adaptive image resize sizing.** Graduated from T9 (Q3). T9 sizes
  to a single conservative target (1568px/5MiB — the strictest provider's own
  threshold); per-model caps (larger for Gemini 3072, tighter for
  cost-sensitive loops) would trade tokens for detail. Sharpen when a
  cost/quality driver appears.

## Out of scope

- **Full pi parity** — posture B chose strategic gap-closing, not full.
- **Confining opi itself via an in-process sandbox** — T4 explicitly excludes
  (needs an external runner — Docker/Gondolin pattern; pi `security.md:35`).
- **TypeScript extension compatibility / `jiti` loading.**
- **npm/gallery, package update/self-update** (cluster G) — Future Ecosystem.
- **web/share, HTML export, gist publishing** (cluster H) — Future Ecosystem.
- **Broad provider catalog (~26 providers, dedicated Mistral/Codex wires)**
  (cluster B) — Future Ecosystem (small dedicated-wire additions may slip in
  opportunistically).
- **pi session v3 read/write compatibility** (`session.rs:58-61`
  `SESSION_FORMAT_POLICY`).
- **Shared `opi-types` crate** (ADR-003).
- **Whole-loop rewrite.**

## Notes on dimensions where opi is ahead (no gap to promote)

- **Dimension 17 (diagnostics/observability):** opi is strictly ahead of pi —
  unified `Diagnostic` model (~51 codes, 9 `SOURCE_` constants), redaction core,
  `TRACE_SCHEMA_VERSION=1` envelope, `opi doctor`. pi has no doctor, no redaction
  core, no shipped trace envelope. No gap flows from pi to opi here.
