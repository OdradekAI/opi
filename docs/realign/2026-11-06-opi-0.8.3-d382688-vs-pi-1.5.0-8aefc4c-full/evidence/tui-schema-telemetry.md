# Batch TUI/SCM/TEL — terminal UI, schema library, telemetry (Dims 9, 19, 17)

Batch summary (5 lines):
1. Contracts measured: pi `packages/tui` (54 src files, ~19.9k lines incl. tests dir separate) vs `crates/opi-tui` (15 modules, 3.6k lines) + `opi-coding-agent/src/interactive.rs` runtime; pi `packages/schema` vs `crates/opi-ai/src/schema/`; pi `packages/telemetry` vs `crates/opi-ai/src/telemetry.rs` + consumers.
2. Verdict tallies (36 records): equivalent 2, partial 8, missing 14, different 11, opi-only 1, unresolved 0.
3. Biggest gaps: pi's own TUI engine (dual render modes, overlay/focus stack, mouse, Kitty keys, IME cursor, alt-screen scrollback/search, editor with undo/kill-ring/history/autocomplete) has no opi counterpart — opi delegates the engine to ratatui+crossterm and ships only stateless-ish widgets; schema is a *different* engine (private TypeBox dialect vs general JSON Schema 2020-12/7 + builders + suite); telemetry is a tracing carrier only (no metrics/propagation/store).
4. Closest matches: coercion pipeline ( opi `normalize.rs` ports pi-ai `validateToolArguments` verbatim-with-documented-divergences), no-eval property, fuzzy select list shape, per-widget markdown/code/diff rendering basics, both telemetry stacks exporting nothing by default.
5. Uncertainties: exact behavior of pi alt-screen internals (layout.ts internals read only via exports); opi interactive runtime assessed from `interactive.rs` only, not from running it.

## A. pi-tui vs opi-tui (+ opi-coding-agent interactive runtime)

pi-tui is a self-contained rendering engine: `Component` contract (`render(width) -> string[]`, optional `handleInput`/`handleMouse`/`invalidate`, `tui.ts:117-149`), own differential renderer with 16 ms min interval and full-redraw counting (`tui.ts:524`, `tui.ts:593` (`fullRedraws` getter)), overlay stack with anchors/percent sizing/focus restore (`tui.ts:260-315`), terminal color-scheme queries OSC 10/11/4+DA1 (`tui.ts:493-497,1526+`), focusable/hardware-cursor IME marker (`CURSOR_MARKER` `tui.ts:202`, `Focusable` `tui.ts:186-198`). opi-tui is a synchronous **ratatui widget library** (`crates/opi-tui/src/lib.rs:1-7`); the engine (raw mode, alt screen, event decode, diffing) is crossterm+ratatui driven from `crates/opi-coding-agent/src/interactive.rs:1183-1210`.

### TUI-ENGINE-01 — TUI engine and component model
- Dimension(s): 9
- pi entry: `packages/tui/src/tui.ts:117-149` (Component `render`/`handleInput`/`handleMouse`/`invalidate`), `tui.ts:470-510` (TUI interface: addChild/setFocus/showOverlay/start/stop/renderNow/addInputListener), `tui.ts:512` (TuiBase) — public + internal
- Opi entry: `crates/opi-tui/src/lib.rs:13-30` (pub mods, no engine types); `crates/opi-coding-agent/src/interactive.rs:1183-1213` (crossterm raw mode + `Terminal::new` + poll loop at 20 fps per doc comment line 4)
- Comparison: different
- Observable difference: pi callers compose `Component`s into a `TUI` with overlays/focus/input-listener registration and observe throttled diff renders and `fullRedraws`; opi callers build a one-shot `Shell` value per frame (`interactive.rs:1919-1953`) and ratatui's `Terminal::draw` owns diffing; no overlay stack (only a single centered picker at `render.rs:139-146`), no focus model, no `TuiStopOptions.preserveScreen`, no exit-restore handler (pi `tui.ts:584` `exitRestoreHandler`, last-resort terminal restore), no terminal color-scheme query/notification.
- Evidence basis: source inspection
- Review outcome: self-reviewed
- Notes: Module-layout delegation alone is not the verdict; the missing overlay/focus/color-query behaviors are caller-observable.

### TUI-MODE-02 — regular (main-screen) vs fullscreen (alt-screen) modes
- Dimension(s): 9
- pi entry: `packages/tui/src/tui.ts:463` (`TuiMode = "regular" | "fullscreen"`); `tui-main-screen.ts:127` (`TuiMainScreen` renders into main screen + scrollback, 1 MiB chunked writer at `tui-main-screen.ts:12-21`); `tui-alt-screen.ts:203` (`TuiAltScreen`, fullscreen) — public
- Opi entry: `crates/opi-coding-agent/src/interactive.rs:1185` (`EnterAlternateScreen`, unconditional single mode), restore at `interactive.rs:1207-1210`; trust prompt re-enters alt screen separately at `interactive.rs:165,200`
- Comparison: different
- Observable difference: pi apps can run inline in the normal scrollback (`regular`) or fullscreen; opi interactive TUI is always fullscreen-alt-screen; there is no inline mode, so terminal scrollback integration differs (opi transcript is bounded by the frame and not scrollable at all — no scroll state anywhere in `interactive.rs` or `message_list.rs`).
- Evidence basis: source inspection
- Review outcome: self-reviewed

### TUI-ALTPORT-03 — alt-screen viewport subsystem (search, scroll, layout, mouse selection)
- Dimension(s): 9
- pi entry: `packages/tui/src/tui-alt-screen.ts:203-1800` (`TuiAltScreen` + `alt-screen-search.ts` (327 lines) transcript search, `layout.ts:280-453` layout engine/scrollbars, `wheel-scroll.ts` acceleration, OSC 133 prompt-jump markers (`tui-alt-screen.ts` imports + `tui-main-screen.ts:12`), synchronized output CSI 2026 (`tui-alt-screen.ts:72-73`)) — public class + internal modules
- Opi entry: bounded absence: `grep -rn "scroll\|offset" crates/opi-coding-agent/src/interactive.rs` → no matches; `grep -rn "search\|Search" crates/opi-tui/src` → no matches; `MessageList` truncates at `inner.height` (`crates/opi-tui/src/message_list.rs:58-66`)
- Comparison: missing
- Observable difference: pi fullscreen users scroll/page/jump between prompts, search the transcript, select text with the mouse; opi users see only the last screenful of messages with no scrolling, search, or selection.
- Evidence basis: source inspection
- Review outcome: self-reviewed

### TUI-KEYS-04 — keyboard input decoding (Kitty protocol, release/repeat)
- Dimension(s): 9
- pi entry: `packages/tui/src/keys.ts:31-38` (kitty active flag), `keys.ts:537` (KeyEventType press/repeat/release), `keys.ts:909` matchesKey, `keys.ts:1344` parseKey, `keys.ts:1456` decodeKittyPrintable; `Key` const map `keys.ts:163` — public
- Opi entry: `crates/opi-tui/src/keybindings.rs:13-21` (`Key` enum: Enter, Escape, Tab, Backspace, Char — 5 variants, no release/repeat/not-modifier-aware keys); crossterm `Event` decode in `interactive.rs:829-830`; absence: `grep -rn "KeyboardEnhancement\|PushKeyboard\|kitty keyboard" crates --include=*.rs` → no matches
- Comparison: different
- Observable difference: pi apps distinguish key press/repeat/release and accept any `KeyId` (arrows, F-keys, word-modifier combos); opi keybindings can only express those 5 keys + ctrl/alt/shift (`keybindings.rs:31-37`), so e.g. `ctrl+left`, `pageUp` defaults, and alt-screen bindings from pi cannot be expressed; no Kitty enhancement negotiation is emitted.
- Evidence basis: source inspection
- Review outcome: self-reviewed

### TUI-KB-05 — keybindings registry/manager
- Dimension(s): 9
- pi entry: `packages/tui/src/keybindings.ts:9-66` (~45 `tui.*` bindings), `:67-161` (`TUI_KEYBINDINGS` defaults with descriptions and multi-key defaults), `:226-333` (`KeybindingsManager`: user config, conflict detection `getConflicts`, resolved bindings, global set/get), `:327` getSelectPageIndex — public
- Opi entry: `crates/opi-tui/src/keybindings.rs:38-45` (`Keybindings{submit,abort,new_line}` only), `:173-203` (builders + `from_config_map` over a 3-key map); no conflicts/resolved-list/descriptions; matched in `interactive.rs:2188-2196`
- Comparison: partial
- Observable difference: both are config-map overridable; pi covers editor/select/alt-screen vocabulary with conflict reporting, opi exposes exactly 3 actions — callers binding `up`/`pageUp`/`ctrl+f` have no opi hook; opi parse accepts only lowercase combos (`keybindings.rs:73-76`).
- Evidence basis: source inspection
- Review outcome: self-reviewed

### TUI-EDITOR-06 — Editor/Input editing components
- Dimension(s): 9
- pi entry: `packages/tui/src/components/editor.ts:297` (`Editor`, 2483 lines: multiline cursor movement, history, undo, kill ring, word navigation, bracketed paste, IME cursor marker, autocomplete providers) and `components/input.ts:41` (`Input`, single-line: cursor, horizontal scroll, paste, kill ring, undo) — public
- Opi entry: `crates/opi-tui/src/editor.rs:14-64` (`InputEditor` renders a static `String`, placeholder only, no cursor/scroll/editing); editing state is a plain `String` in `interactive.rs:1624-1630` (`push(c)` / `pop()` only); absence: `grep -rn "undo\|kill\|yank\|grapheme\|paste" crates/opi-tui/src` → no matches
- Comparison: missing
- Observable difference: pi users edit anywhere in multi-line text (cursor keys, ctrl+a/e/k/w/y, undo ctrl+-, history up/down, word jumps alt+b/f); opi users can only append/delete the last character, and there is no input history, no undo, no paste handling, no visible cursor position.
- Evidence basis: source inspection
- Review outcome: self-reviewed
- Notes: `EditorComponent` interface for custom editors (`editor-component.ts:74`) likewise absent.

### TUI-COMP-07 — component inventory mapping
- Dimension(s): 9
- pi entry: `packages/tui/src/index.ts:40-78` (18 public components)
- Opi entry: `crates/opi-tui/src/lib.rs:13-30` (14 modules)
- Comparison: partial
- Mapping (pi component → opi counterpart):
  | pi (index.ts line) | opi | note |
  |---|---|---|
  | Box :40 | — | absence (ratatui `Block::bordered` used inline) |
  | CancellableLoader :41 | — | absence |
  | Editor :42 | `editor.rs` display-only | see TUI-EDITOR-06 |
  | HStack :43 | — | absence (ratatui `Layout`) |
  | Image :44-49 | `message_list.rs:92-104` escape emission | see TUI-IMG-09 |
  | Input :51 | — | absence (folded into TUI-EDITOR-06) |
  | Loader :52 | — | absence |
  | Markdown :53 | `markdown.rs:149` | see TUI-MD-08 |
  | MouseRegion :54 | — | absence (no mouse) |
  | ScrollView :55-60 | — | absence |
  | SelectList :61-67 | `select_list.rs:322` | see TUI-SELECT-18 |
  | SettingsList :68 | — | absence (no settings UI widget) |
  | Spacer :69 | — | absence |
  | Text :70 | — | absence (plain `Span`/`Line` inline) |
  | TruncatedText :71 | — | absence |
  | VStack :72-78 | — | absence |
- Observable difference: only Editor/Markdown/SelectList(+Image via MessageList) have opi counterparts; the other 13 pi widgets have no opi equivalent, and opi's stack/flex widgets come from ratatui at the call site rather than the crate.
- Evidence basis: source inspection
- Review outcome: self-reviewed

### TUI-MD-08 — Markdown rendering
- Dimension(s): 9
- pi entry: `packages/tui/src/components/markdown.ts:279` (1120 lines over `marked` tokens: headings all depths, GFM tables (`markdown.ts:114-116`), lists/blockquote, streaming partial code fences (`markdown.ts:185-190`), `highlightCode` hook (`markdown.ts:247`), LaTeX inline/block (`markdown.ts:38-147`), theme fns + caching/invalidation) — public
- Opi entry: `crates/opi-tui/src/markdown.rs:168-202` (hand parser: fenced code blocks, `#`-headings only h1-h3 styled at `:238-255`, paragraphs joined; `CodeBlock` at `:14` with language title, no highlighting)
- Comparison: partial
- Observable difference: pi renders lists, tables, blockquotes, emphasis, links and LaTeX and keeps streaming code blocks stable; opi renders headings/paragraphs/code-fences only — a bullet list or table renders as plain paragraph text; headings deeper than h3 fall back to h3 styling; no syntax highlighting hook.
- Evidence basis: source inspection
- Review outcome: self-reviewed

### TUI-IMG-09 — terminal image support
- Dimension(s): 9
- pi entry: `packages/tui/src/terminal-image.ts` (783 lines): capability detection incl. tmux (`:157`), get/set cell dimensions (`:41-45`), kitty encode with placement/crop/delete (`:237-501`), iTerm2 encode (`:304`), PNG/JPEG/GIF/WebP dimension parsing (`:549-688`), renderImage (`:690`), hyperlink (`:752`), imageFallback (`:770`) — public
- Opi entry: `crates/opi-tui/src/terminal_image.rs`: `detect_graphics_protocol` env-var only (`:65-90`: `TERM=xterm-kitty` / `TERM_PROGRAM=iTerm.app`, `TERM_FEATURES` ignored), `kitty_escape` PNG-only `a=T,f=100` (`:92-106`), `iterm_escape` (`:108-125`), `sixel_escape` returns empty string (`:127-129`), `text_fallback` (`:134-139`)
- Comparison: partial
- Observable difference: opi detects fewer terminals (no tmux/DA1 probing, no capability cache/overrides), cannot parse image dimensions (caller must supply `width/height` for the fallback label), emits no kitty placement/scroll/crop or delete sequences, never actually renders Sixel (empty string) — an image in a Sixel terminal shows only `[Image: …]` text; kitty path requires PNG.
- Evidence basis: source inspection
- Review outcome: self-reviewed

### TUI-THEME-10 — colors, styles, terminal color scheme
- Dimension(s): 9
- pi entry: `packages/tui/src/colors.ts` (367 lines: OKLCH/OKHSL color model `:16-107`, mixing `:241`, indexed/truecolor ANSI `:309-330`, parseColor `:121`), `oklab.ts`, `terminal-colors.ts:91` (`parseTerminalColorSchemeReport`), scheme queries in `tui.ts:493-497` + change notification; per-component `*Theme` interfaces — public
- Opi entry: `crates/opi-tui/src/theme.rs`: `parse_color` named+hex only (`:49-89`), 32 semantic tokens `THEME_TOKENS` (`:96-124`), `Theme` struct with built-ins default/monokai (`:275,327`), `from_color_map` (`:327`), `resolve_theme` (`:428`); absence: `grep -rn "oklch\|Oklch\|OSC 10\|\]10;" crates/opi-tui/src` → no matches
- Comparison: different
- Observable difference: pi themes are computed in a perceptual color space, can express indexed-256 colors and mix colors, and adapt to the terminal's reported palette (light/dark) via OSC 10/11/4 queries; opi themes are fixed named/hex RGB values per widget role with no palette awareness, no dynamic scheme switching, no indexed-color support.
- Evidence basis: source inspection
- Review outcome: self-reviewed
- Notes: opi's theme file discovery lives in `crates/opi-coding-agent/src/theme_discovery.rs:1-26` (TOML, token names) — belongs to the coding-agent batch.

### TUI-UTIL-11 — width/wrap/grapheme/OSC8 utilities
- Dimension(s): 9
- pi entry: `packages/tui/src/utils.ts` (1467 lines: `visibleWidth`, `wrapTextWithAnsi`, `sliceByColumn`, `stripTerminalSequences`, `truncateToWidth`, `getOsc8LinkAtColumn`, grapheme/word segmenters) exported `index.ts:187-197` — public
- Opi entry: absence: `grep -rn "visibleWidth\|wrapText\|osc8\|hyperlink\|grapheme" crates/opi-tui/src` → only `unicode_width` used in `branch_picker.rs:9`; ratatui handles width internally per widget
- Comparison: missing
- Observable difference: pi components preserve ANSI styling through wrapping/truncation and detect OSC 8 links under the cursor; opi widgets truncate naively (e.g. `MessageList` cuts whole messages, `MarkdownView` breaks at area height) and never produce or detect hyperlinks.
- Evidence basis: source inspection
- Review outcome: self-reviewed

### TUI-AUTO-12 — autocomplete (paths + slash commands)
- Dimension(s): 9
- pi entry: `packages/tui/src/autocomplete.ts:246-268+` (AutocompleteProvider/Suggestions, SlashCommand, `CombinedAutocompleteProvider`, fd-based path search `:1-50`) — public
- Opi entry: absence: `grep -rni "autocomplete" crates/opi-tui/src crates/opi-coding-agent/src` → no matches
- Comparison: missing
- Observable difference: pi editors offer path/slash-command completions while typing; opi has none.
- Evidence basis: source inspection
- Review outcome: self-reviewed

### TUI-LATEX-13 — LaTeX rendering
- Dimension(s): 9
- pi entry: `packages/tui/src/latex.ts:1488` `renderLatex` (1506 lines, Unicode text output), used by markdown `markdown.ts:3` — public
- Opi entry: absence: `grep -rni "latex" crates/opi-tui/src crates/opi-coding-agent/src` → no matches
- Comparison: missing
- Observable difference: pi renders `$…$`/`$$…$$` math as Unicode text in markdown; opi shows the raw LaTeX source.
- Evidence basis: source inspection
- Review outcome: self-reviewed

### TUI-TERM-14 — Terminal abstraction, stdin buffering, Apple quirks
- Dimension(s): 9
- pi entry: `packages/tui/src/terminal.ts:86` (`Terminal` interface incl. `restoreOnExit`), `:164` `ProcessTerminal` (639 lines: kitty keyboard negotiation parse `:24-43`, Apple session detection `:45`, input normalization `:68-84`, escape timeout `:150`), `stdin-buffer.ts:305` (`StdinBuffer` batch splitting) — public
- Opi entry: crossterm backend used directly (`interactive.rs:17-21,1186-1188`); no Terminal abstraction, no stdin batching, no Apple-specific normalization in-tree
- Comparison: different
- Observable difference: pi can swap terminal implementations (tests use fakes), buffers split input batches and normalizes Apple-Terminal shift-enter; opi behavior is whatever crossterm provides — kitty negotiation, paste splitting and platform quirks are delegated and not caller-controllable.
- Evidence basis: source inspection
- Review outcome: self-reviewed

### TUI-PROGSTAT-15 — OSC 7501 program status
- Dimension(s): 9
- pi entry: `packages/tui/src/program-status.ts:14-54` (`ProgramStatus` state/app/kind/msg, `formatProgramStatus`, query/reply detection) — public
- Opi entry: absence: `grep -rn "7501\|ProgramStatus" crates --include=*.rs` → no matches
- Comparison: missing
- Observable difference: pi reports idle/working/blocked/done/error to supporting terminals (OSC 7501); opi terminals never receive program status.
- Evidence basis: source inspection
- Review outcome: self-reviewed

### TUI-CLIP-16 — native clipboard
- Dimension(s): 9
- pi entry: `packages/tui/src/native-platform.ts` (`getNativeClipboard`, 65 lines + `native-module-path.ts`, `native-modifiers.ts`) — public
- Opi entry: absence: `grep -rni "clipboard" crates/opi-tui/src crates/opi-coding-agent/src` → no matches
- Comparison: missing
- Observable difference: pi can read the OS clipboard (for paste/copy selection); opi cannot.
- Evidence basis: source inspection
- Review outcome: self-reviewed

### TUI-MOUSE-17 — mouse events, regions, wheel
- Dimension(s): 9
- pi entry: `packages/tui/src/tui.ts:25-120` (normalized `TuiMouseEvent`, dispatch/capture/focus results), `components/mouse-region.ts`, `wheel-scroll.ts` (accelerator), alt-screen selection — public
- Opi entry: absence: `grep -rn "Mouse\|EnableMouseCapture" crates/opi-tui/src crates/opi-coding-agent/src` → no matches
- Comparison: missing
- Observable difference: pi apps handle click/drag/wheel with focus and capture routing; opi ignores all mouse input (mouse capture never enabled).
- Evidence basis: source inspection
- Review outcome: self-reviewed

### TUI-SELECT-18 — SelectList + fuzzy
- Dimension(s): 9
- pi entry: `packages/tui/src/components/select-list.ts:40` (281 lines: themes, `truncatePrimary` context, layout options, page keys via keybindings, mouse), `fuzzy.ts:88,121` (`fuzzyMatch` with alpha-digit swap fallback, `fuzzyFilter` multi-token whitespace/slash) — public
- Opi entry: `crates/opi-tui/src/select_list.rs:322` (`SelectList` widget), `:123` (`SelectListState`: fuzzy filter over display+metadata `:189-256`, move/page/confirm, scroll window), `:39` `fuzzy_match` greedy forward with gap penalty `:76-99`; driven by `handle_picker_key` (`interactive.rs:2089-2143`)
- Comparison: partial
- Observable difference: both offer filterable, paged, themed selection; pi additionally supports mouse selection, keybinding-driven paging and smarter fuzzy (multi-token AND, digit-swap fallback, subsequence scoring parity); opi fuzzy is single-token greedy left-to-right, so `md file` or `p1` vs `1p` behave differently.
- Evidence basis: source inspection
- Review outcome: self-reviewed

### TUI-OPI-19 — opi-only widgets (no pi-tui counterpart)
- Dimension(s): 9
- pi entry: none in `packages/tui/src/index.ts` (closest pi equivalents live in `packages/coding-agent` UI code, outside this package)
- Opi entry: `crates/opi-tui/src/message_list.rs:25`, `status_bar.rs:15` (model/state/tokens/cost), `tool_call.rs:15`, `diff_view.rs:40` (unified diff with own LCS `compute_diff` `:65`), `permission_prompt.rs`, `trust_prompt.rs`, `branch_picker.rs:22`
- Comparison: opi-only
- Observable difference: these widgets exist only in opi (transcript list, status bar, tool-call box, diff view, trust/permission prompts, branch picker); in pi the same UX is delivered by coding-agent-layer components, so within the tui-package scope they are additions.
- Evidence basis: source inspection
- Review outcome: self-reviewed
- Notes: cross-package comparison of those coding-agent UIs belongs to the coding-agent batch.

## B. pi-schema vs opi-ai schema

pi-schema is a general JSON Schema library: drafts 2020-12 + 7, 2222/2223 official-suite claim (`packages/schema/README.md:6-7`, vendored `test/fixtures/json-schema-test-suite`), builder DSL returning plain JSON Schema (`src/builders.ts:44-179`), `compile()` → closure `Validator` with `check`/`errors`/`coerce`/`parse` (`src/validator.ts:32-76`), JSON-Pointer errors `{path, keyword, message}` (`src/types.ts:39-43`). opi-ai's `schema` module is explicitly **not** that: "the semantics of the pinned TypeBox 1.3.27 `Compile` evaluation path … deliberately not a general JSON Schema validator and not a public conformance API" (`crates/opi-ai/src/schema/mod.rs:1-17`).

### SCM-DRAFT-01 — draft coverage (2020-12 + 7)
- Dimension(s): 19
- pi entry: `packages/schema/src/types.ts:15` (`draft?: "2020-12" | "7"`), `types.ts:22,30` (external `schemas` map, `unresolvedRefs`), keyword coverage list `README.md:272-280` (incl. draft-7 `dependencies`/`items` arrays, `$dynamicRef`, `unevaluated*`) — public
- Opi entry: `crates/opi-ai/src/schema/mod.rs:3-9` (fixed private TypeBox dialect; no draft parameter anywhere: `grep -rn "draft" crates/opi-ai/src/schema` → only `refs.rs` comment); keyword set is the pinned TypeBox structural set (`guards.rs:1-14`)
- Comparison: different
- Observable difference: a caller validating the same schema+instance through both libraries gets different verdicts on draft-specific keywords (e.g. draft-7 `dependencies`, `$recursiveRef` vs `$dynamicRef` handling, `$vocabulary`): pi documents/validates them; opi's dialect evaluates the TypeBox `Compile` subset only, and offers no draft selection.
- Evidence basis: source inspection
- Review outcome: self-reviewed

### SCM-BUILD-02 — builder DSL (obj/str/int/opt/oneOf/…)
- Dimension(s): 19
- pi entry: `packages/schema/src/builders.ts:44-179` (`str/num/int/bool/nul/unknown/lit/oneOf/union/nullable/arr/tuple/map/bytes/Optional/opt/obj` + annotations) and `Static` typing — public
- Opi entry: absence: `grep -rn "pub fn obj\|pub fn str\|pub fn int\|Optional\|pub fn lit" crates/opi-ai/src/schema crates/opi-ai/src | head` → no builder API; opi builds schemas as `serde_json::Value` literals at call sites
- Comparison: missing
- Observable difference: pi callers construct typed schemas that both type-check values and serialize to tool JSON Schema; opi callers hand-write JSON objects with no compile-time type derivation.
- Evidence basis: source inspection
- Review outcome: self-reviewed

### SCM-COMPILE-03 — compile()/validator contract
- Dimension(s): 19
- pi entry: `packages/schema/src/validator.ts:32-76` — `compile(schema, options)` → `Validator.check(value): boolean` fast path + lazily built `errors(value)`; options `formats`/`schemas`/`unresolvedRefs`/`coerce` (`types.ts:16-33`) — public
- Opi entry: `crates/opi-ai/src/schema/mod.rs:144-176` — `compile(Value) -> Result<CompiledSchema, SchemaError>` + `check(&Value) -> Result<bool, SchemaError>` (typed engine errors `SchemaError` `mod.rs:91-119`); no options; `errors`-equivalent `check_detailed` is `pub(crate)` (`mod.rs:168-176`)
- Comparison: partial
- Observable difference: both compile once and check cheaply; pi exposes failure explanation and reference-document/format options on the public API, opi's public seam is boolean `check` plus the tool-argument wrapper (SCM-TOOLVAL-10) — a general library consumer cannot get error details from `opi_ai::schema` directly.
- Evidence basis: source inspection
- Review outcome: self-reviewed

### SCM-ERROR-04 — error format (JSON Pointer, all failures)
- Dimension(s): 19
- pi entry: `packages/schema/src/types.ts:39-43` (`{path, keyword, message}` per failure, `errors()` lists every field failure, README example `README.md:24-30`); `explain.ts` builds them (with `$ref` branch blame memoization `explain.ts:26-27`) — public
- Opi entry: `crates/opi-ai/src/schema/eval.rs` first-failure site only via `check_detailed` (`mod.rs:166-176`); surfaced as `ToolArgumentFailure {instance_path, keyword, reason}` (`normalize.rs:41-46`) with redaction rules; agent-level string format `"{path}: {keyword}: {reason}"` pointer-prefixed (`crates/opi-agent/src/validation.rs:62-70`)
- Comparison: different
- Observable difference: pi reports all failing fields at once with human messages; opi reports only the first observed failure site with a schema-derived reason, and pi-ai converts pointers to dotted model-facing paths (`edits.0.oldText`, pi `ai/src/utils/validation.ts:20-27`) while opi keeps raw JSON-pointer form (`/edits/0/oldText`) — models see differently shaped feedback.
- Evidence basis: source inspection
- Review outcome: self-reviewed

### SCM-FORMATS-05 — string formats
- Dimension(s): 19
- pi entry: `packages/schema/src/formats/index.ts:1-25` — all 2020-12 formats + `url` + `json-pointer-uri-fragment` (24 checks), optional entry point injected via `compile({formats})`, else annotation-only per 2020-12; custom format map supported — public
- Opi entry: `crates/opi-ai/src/schema/formats.rs:1-31` — 21 fixed TypeBox-build formats, unknown names inert (`:515-517`), not injectable, no option to treat `format` as assertion vs annotation
- Comparison: different
- Observable difference: pi callers choose whether `format` asserts and can add custom formats; in opi `format` always behaves as the pinned TypeBox registry (a fixed subset, e.g. no separate `url` vs `uri` distinction beyond the ported set) with no caller control.
- Evidence basis: source inspection
- Review outcome: self-reviewed

### SCM-REFS-06 — $ref/resolution model
- Dimension(s): 19
- pi entry: `packages/schema/README.md:275-277` + `types.ts:22-31` — `$id`/`$anchor`/local-relative-remote `$ref` via `schemas` option, `$dynamicRef`/`$dynamicAnchor`, `unresolvedRefs: "throw"|"reject"` — public
- Opi entry: `crates/opi-ai/src/schema/refs.rs` (69 fns: `$ref`/`$recursiveRef`/`$dynamicRef` frames `:36-94,375-381`, WHATWG URI/fragment hashing `hashing.rs`, punycode/IDNA `punycode.rs`); no external document map, no unresolvedRefs policy switch
- Comparison: different
- Observable difference: pi validates schemas spanning multiple documents and can reject-at-place on dangling refs (used for MCP tool schemas, pi `ai/src/utils/validation.ts:16`); opi resolves only within the single admitted document and a dangling `$ref` is a typed `Uri`/compile error instead of a place-local rejection.
- Evidence basis: source inspection
- Review outcome: self-reviewed

### SCM-COERCE-07 — coercion pipeline
- Dimension(s): 19
- pi entry: `README.md:107-160` (3 passes: optional-null deletion, TypeBox `Value.Convert` keyed conversions, Ajv-style primitive coercion; `coerce()`/`parse()` on `CoercingValidator` `validator.ts:58-76`) — public
- Opi entry: `crates/opi-ai/src/schema/normalize.rs:1-115` — ports the pi-ai `validateToolArguments` pipeline: arguments_error pre-rejection, clone-before-normalize, optional-null deletion with sub-compile caching, union/allOf/anyOf/oneOf coercion, type-array coercion, final root check; documents divergences (TypeBox `~kind` no-op `:19-21`; coerced-root-primitive failure reports Invalid `:23-25` instead of upstream's unvalidated-raw return)
- Comparison: partial
- Observable difference: for the tool-arguments use case the observable outcomes match the pinned pi corpus except the two documented divergences; as a general library feature, opi has no public `coerce()`/`parse()` — coercion exists only inside `validate_tool_arguments`.
- Evidence basis: source inspection + test inspection (`tests/schema_public.rs:416,453,474,569-909` corpus + divergence tests)
- Review outcome: self-reviewed

### SCM-SUITE-08 — official JSON Schema test suite conformance
- Dimension(s): 19
- pi entry: `packages/schema/README.md:6-7` (2,222/2,223 core tests), vendored suite `test/fixtures/json-schema-test-suite/{tests,remotes,meta-schemas}`, conformance runner `test/conformance.test.ts`, `test/suite.ts` — public claim + tooling
- Opi entry: absence: `grep -rn "json-schema-test-suite\|2222\|conformance" crates/opi-ai/tests crates/opi-ai/src/schema` → no suite; instead 64-case frozen differential corpus from research probes (`tests/schema_engine.rs:1-14`) + 79 focused tests + 30 public-seam tests (`tests/schema_public.rs`)
- Comparison: missing
- Observable difference: pi's claim is independently checkable against the official suite (fixtures in-tree); opi conformance is only argued relative to the pinned TypeBox dialect via a 64-case corpus, so draft-suite behavior cannot be verified or claimed.
- Evidence basis: source inspection + test inspection
- Review outcome: self-reviewed

### SCM-NOEVAL-09 — no eval / CSP-safe execution
- Dimension(s): 19
- pi entry: `packages/schema/src/index.ts:1-2` ("without eval … compiled closures"), README:2-4 — public claim
- Opi entry: `crates/opi-ai/src/schema/mod.rs:5-7` ("explicit work-stack machine … no implicit I/O", `pump.rs` iterative evaluation), typed `SchemaError::NoProgress` for unbounded recursion (`mod.rs:107-111`)
- Comparison: equivalent
- Observable difference: none at this property level: neither executes generated code; opi additionally converts pi/TypeBox's would-be stack exhaustion into a typed error (stricter, same observable "no eval" contract).
- Evidence basis: source inspection
- Review outcome: self-reviewed

### SCM-TOOLVAL-10 — tool-argument validation seam (the real consumer)
- Dimension(s): 19
- pi entry: `packages/ai/src/utils/validation.ts:8-54` (`validateToolCall`/`validateToolArguments` with WeakMap validator cache, `compile({formats, coerce:true, unresolvedRefs:"reject"})`); consumed by `packages/agent/src/agent-loop.ts` and `packages/durable/src/harness/tool.ts`; typing via `Static` in `packages/agent/src/types.ts:17,466` — public
- Opi entry: `crates/opi-ai/src/schema/normalize.rs:63-128` (`validate_tool_arguments`, `validate_tool_call` by name), consumed by `crates/opi-agent/src/validation.rs:23-77` (`ToolSchemaIr` compile-once per registration, fail-closed on admission error, redacted string errors)
- Comparison: partial
- Observable difference: same pipeline shape (name lookup → typed unknown-tool rejection → coerce+check → normalized value on success); opi adds explicit `ArgumentsError` pre-rejection state and fail-closed admission errors, redacts reasons, and emits pointer-style paths (see SCM-ERROR-04); pi caches validators per schema object identity, opi compiles per tool registration.
- Evidence basis: source inspection + test inspection
- Review outcome: self-reviewed

## C. pi-telemetry vs opi-ai telemetry

pi-telemetry is a vendor-neutral callback contract: `TelemetryContext.startSpan(options, callback)` with metrics/propagation capabilities, `NOOP_*` singletons, typed schema DSL, InMemory reference, store queries, render + conformance kit (`packages/telemetry/src/index.ts:98-109`, `metrics.ts:31-145`, `noop.ts:14-37`, `memory.ts`, `render.ts:11-49`, `testing/`). opi-ai's `telemetry.rs` (155 lines) is a *process-local tracing carrier*: frozen `(Span, Dispatch)` with `in_scope`/`instrument` and explicit noop state (`crates/opi-ai/src/telemetry.rs:54-131`), explicitly non-goal'd against the generic span/schema DSL (`:41-49`).

### TEL-CTX-01 — TelemetryContext/Span contract + NOOP
- Dimension(s): 17
- pi entry: `packages/telemetry/src/index.ts:98-109` (TelemetryContext/TelemetrySpan: startSpan callback, addEvent/setAttributes/setStatus/addLink), `noop.ts:14-37` (NOOP_TELEMETRY_CONTEXT/METRICS/PROPAGATION) — public
- Opi entry: `crates/opi-ai/src/telemetry.rs:54-131` (`TelemetryContext::from_span` `:73`/`noop` `:85`/`in_scope` `:93`/`instrument` `:102`); no span lifecycle API — spans are created by callers through `tracing` macros and passed in
- Comparison: different
- Observable difference: pi code opens child spans through the context and records events/attributes/status per span; opi code can only *scope* existing work under a frozen parent — creating spans, setting status or adding events happens outside the contract, so an adapter cannot observe span events through this type at all.
- Evidence basis: source inspection
- Review outcome: self-reviewed

### TEL-PROP-02 — propagation (inject/extract) and span links
- Dimension(s): 17
- pi entry: `packages/telemetry/src/index.ts:17-33` (`TelemetryPropagation.inject/extract` over opaque carriers), `SpanLink` `:35-39` — public
- Opi entry: absence: `grep -rn "inject\|extract\|SpanLink\|link" crates/opi-ai/src/telemetry.rs` → no matches; doc states "process-local, never serialized, never placed on the wire" (`telemetry.rs:9-10`)
- Comparison: missing
- Observable difference: pi distributes trace context across processes/adapters and links spans; opi cannot — cross-process correlation is out of contract.
- Evidence basis: source inspection
- Review outcome: self-reviewed

### TEL-METRICS-03 — counters/up-down counters/histograms
- Dimension(s): 17
- pi entry: `packages/telemetry/src/metrics.ts:31-45` (`TelemetryMetrics.counter/upDownCounter/histogram` with units/boundaries), attached to contexts (`index.ts:99-101`) — public
- Opi entry: absence: `grep -rn "counter\|histogram\|metric" crates/opi-ai/src/telemetry.rs` → no matches
- Comparison: missing
- Observable difference: pi records request-duration/first-chunk histograms etc. (pi `ai/src/telemetry.ts:63-77`); opi emits no metrics anywhere in-tree.
- Evidence basis: source inspection
- Review outcome: self-reviewed

### TEL-SCHEMA-04 — typed schema DSL (defineTelemetrySchema / typed starters)
- Dimension(s): 17
- pi entry: `packages/telemetry/src/index.ts:112-465` (`defineTelemetrySchema`, attribute/event/parent definitions, `createTypedSpanStarter` `:420-437`, `createTypedMetrics` `metrics.ts:128-145`) — public
- Opi entry: absence: `grep -rn "Schema\|Typed" crates/opi-ai/src/telemetry.rs` → only the explicit non-goal comment (`:47-49` "no domain span names … no claim of equivalence to a generic span callback/schema DSL")
- Comparison: missing
- Observable difference: pi packages publish machine-readable span/metric vocabularies with compile-time attribute typing (e.g. `AI_TELEMETRY_SCHEMA` pi `ai/src/telemetry.ts:13-80`); opi has no vocabulary or typing layer.
- Evidence basis: source inspection
- Review outcome: self-reviewed

### TEL-INMEM-05 — InMemory reference, store queries, render, conformance kit
- Dimension(s): 17
- pi entry: `packages/telemetry/src/memory.ts:1-60+` (`InMemoryTelemetryContext`, `RecordedTelemetrySpan/Measurement`, `TelemetryStore.findTraces/getTrace/findMetrics` queries `index.ts:55-96`), `render.ts:11` (`renderTrace` forest text), `testing/conformance.ts` (adapter conformance cases) — public
- Opi entry: absence: no equivalent in `crates/opi-ai/src/telemetry.rs` (155 lines total); test-side recording dispatch exists only inside `crates/opi-ai/tests/codex_telemetry.rs:34-93` as a local `tracing` subscriber fixture
- Comparison: missing
- Observable difference: pi ships a runnable in-memory backend usable for tests and debugging with queryable traces and a conformance kit for adapter authors; opi tests must hand-roll tracing subscribers and nothing queries stored telemetry.
- Evidence basis: source inspection + test inspection
- Review outcome: self-reviewed

### TEL-CONSUMER-06 — provider/agent wiring of telemetry
- Dimension(s): 17
- pi entry: `packages/ai/src/types.ts:162-163` (`telemetry?: TelemetrySpan` on request), `ai/src/telemetry.ts:64-80` (typed `pi.ai.request`/`pi.ai.poll` spans + duration/first-chunk histograms with usage/cost attributes) — public
- Opi entry: `crates/opi-ai/src/provider.rs:304` (`telemetry_context: Option<TelemetryContext>` on request), producer tasks spawned under `context.instrument(producer)` (`anthropic.rs:2109`, `azure_openai.rs:546,784`, `bedrock/mod.rs:749`, `mistral.rs:903`, …); attempts awaited under it (`api/decisions.rs:493-496`, `api/llama.rs:660-661`); no span/metric vocabulary recorded
- Comparison: different
- Observable difference: both thread an explicit parent through requests (no ambient capture); pi observers get named spans with usage/cost/stop attributes and histograms, opi observers only see whatever the host's tracing subscriber already captures around the producer task — the opi context itself emits nothing.
- Evidence basis: source inspection + test inspection (`tests/codex_telemetry.rs` verifies frozen-dispatch scoping/enter counts)
- Review outcome: self-reviewed

### TEL-EXPORT-07 — exporters / global state
- Dimension(s): 17
- pi entry: `packages/telemetry/README.md:9-11` ("no exporter, global current-span state, or dependency on a telemetry backend" — deliberate non-goal; adapters host-owned) — public claim
- Opi entry: `crates/opi-ai/src/telemetry.rs:41-44` ("No global subscriber initialization, exporter, sampling or flush (host owned)"); binary installs no subscriber (`grep -rn "tracing_subscriber\|fmt().init" crates/opi-coding-agent/src` → no matches; only `tracing::debug!` calls e.g. `adapter_host.rs:232`)
- Comparison: equivalent
- Observable difference: none by design — both deliberately ship no exporter or global state and leave observation to the host; opi's host today wires no subscriber, so both stacks are silent by default.
- Evidence basis: source inspection
- Review outcome: self-reviewed

## Layer H — active design material encountered

| Claim | Anchor | Authority wording | Observed maturity |
|---|---|---|---|
| "Theme discovery types are part of the unstable 0.x extension API" | `crates/opi-tui/src/theme.rs:23-26` | crate doc, normative for 0.x | implemented |
| opi schema is "deliberately not a general JSON Schema validator … Complete conformance and final distribution acceptance remain separate gates" | `crates/opi-ai/src/schema/mod.rs:8-17` | crate doc constraint | implemented as dialect; the "separate gates" are open work items |
| opi telemetry "no claim of equivalence to a generic span callback/schema DSL" + host-owned exporters | `crates/opi-ai/src/telemetry.rs:41-49` | crate doc non-goals | implemented (carrier only) |
| pi-tui "Minimal terminal UI framework with differential rendering and synchronized output for flicker-free interactive CLI applications" | `.pi/pi-1.5.0/packages/tui/README.md:1-15` | package README (descriptive) | implemented (test dir has 54 entries incl. `exit-restore`, `overlay-*`, mouse, latex, keys suites) |
| pi-schema "Passes 2,222 of the 2,223 core tests" + builder/coercion/format docs | `.pi/pi-1.5.0/packages/schema/README.md:2-160` | package README (descriptive, backed by vendored suite fixtures) | implemented |
| pi-telemetry "no exporter, global current-span state, or dependency on a telemetry backend" | `.pi/pi-1.5.0/packages/telemetry/README.md:9-11` | package README non-goal | implemented |

## Verdict tallies

| Verdict | TUI | SCM | TEL | Total |
|---|---|---|---|---|
| equivalent | 0 | 1 (NOEVAL) | 1 (EXPORT) | 2 |
| partial | 5 (KB, COMP, MD, IMG, SELECT) | 3 (COMPILE, COERCE, TOOLVAL) | 0 | 8 |
| missing | 8 (ALTPORT, EDITOR, UTIL, AUTO, LATEX, PROGSTAT, CLIP, MOUSE) | 2 (BUILD, SUITE) | 4 (PROP, METRICS, SCHEMA, INMEM) | 14 |
| different | 5 (ENGINE, MODE, KEYS, THEME, TERM) | 4 (DRAFT, ERROR, FORMATS, REFS) | 2 (CTX, CONSUMER) | 11 |
| opi-only | 1 (OPI-19) | 0 | 0 | 1 |
| unresolved | 0 | 0 | 0 | 0 |

## Coverage honesty

- Did NOT run either tree (read-only constraint); all verdicts are source/test-inspection based, no executed probes.
- pi-tui internals read selectively: full `index.ts`, `keybindings.ts`, large parts of `tui.ts` (0-700), headers/exports of `tui-alt-screen.ts`, `tui-main-screen.ts` (1-40), `keys.ts`, `terminal.ts`, `terminal-image.ts` (exports), `colors.ts` (exports), `program-status.ts` (full), `autocomplete.ts` (1-50), `input.ts` (1-60), `settings-list.ts` (1-60), `markdown.ts` (grep-level), `select-list.ts`/`fuzzy.ts` (fuzzy bodies). NOT inspected: `latex.ts` internals (1506 lines), `utils.ts` internals, `layout.ts` internals, `editor.ts` body beyond exports, `stdin-buffer.ts` body, `memory.ts` beyond line 60 — treated at export/claim level.
- opi runtime assessed from `interactive.rs` (2774 lines): read event loop (1180-1660), key handling (1370-1640, 2085-2200), shell build (1919-1953), trust prompt plumbing (111-290); NOT read line-by-line: session/picker command handling (1400-1600), test scaffolding (2200-2774), `permission.rs`/`runtime.rs` beyond telemetry greps.
- pi schema: `index.ts`/`types.ts`/`validator.ts` full, `builders.ts` exports, `formats/index.ts` full, README full; NOT inspected: `compiler.ts`, `resolver.ts`, `checks.ts`, `coerce.ts`, `explain.ts`, `annotations.ts`, `typebox-static.ts` bodies, bench/ and the vendored suite contents (file lists only).
- pi telemetry: `index.ts` full (465), `metrics.ts` full, `noop.ts` full, `render.ts` full, `memory.ts` 1-60, `testing/*` headers; NOT inspected: `memory.ts` 61-463 details, conformance case bodies.
- opi schema: `mod.rs`/`normalize.rs` full headers+key sections, `guards.rs` 1-50, `formats.rs` header+greps, `refs.rs` greps; NOT line-read: `eval.rs`, `ir.rs`, `admission.rs`, `proof.rs`, `pump.rs`, `js.rs`, `grapheme.rs`, `hashing.rs`, `punycode.rs` (characterized from module docs and tests).
- Ratatui/crossterm/TypeBox library internals are third-party and out of tree; behavior claims about them rest on how opi calls them, not their source.
- Absence searches used: `grep -rn` over `crates/` for `7501|ProgramStatus`, `KeyboardEnhancement|PushKeyboard|kitty keyboard`, `EnableMouseCapture|MouseEvent`, `EnableBracketedPaste`, `clipboard`, `latex`, `undo|kill|yank|grapheme|paste`, `osc8|hyperlink`, `tracing_subscriber`, `scroll|offset` (interactive.rs), `autocomplete`, `inject|extract|SpanLink`, `counter|histogram|metric` (telemetry.rs) — each returned no matches outside the noted files.
