# Batch CLI/SES — CLI, modes, sessions, export (Dims 11, 6, 13)

Measured 2026-11-06 against inputs in `00-inputs.md` (pi 1.5.0 export `8aefc4c…`,
Opi `d382688` clean). Strict reproduction lens: `equivalent | partial | missing |
different | opi-only | unresolved`.

Batch summary (5 lines):
1. Contracts measured: 32 records — CLI flag surface (args.ts usage vs clap
   `cli.rs`), four run modes (interactive/print/json/rpc), session storage
   semantics, session export (HTML/JSONL/markdown), auth/startup commands.
2. Verdict tallies: equivalent 0 · partial 11 · missing 9 · different 11 ·
   opi-only 1 · unresolved 0.
3. Biggest gaps: all pi resource-discovery flags (`--extension/--skill/
   --prompt-template/--theme/--no-context-files/…`) and `--no-session/
   --session/--session-id/--session-dir/--name/--continue` have no opi flag;
   RPC is a 12-command subset with a different event wire; HTML export absent.
4. Cross-cutting differences: no piped-stdin/TTY auto-print; session files are
   eagerly created flat-dir `{hexmillis}.jsonl` v2 (not pi v3 layout, by
   declared policy `SESSION_FORMAT_POLICY`); `--export` HTML vs
   `--export-session` markdown/json.
5. Uncertainties: pi TUI internals (theme engine, widgets) only sampled — the
   interactive-mode deep comparison belongs to the interactive/TUI batch; exit
   codes for print mode were read from source, not executed.

---

### CLI-01 — CLI parser, entry and help surface
- Dimension(s): 11
- pi entry: `packages/coding-agent/src/cli.ts:1-6` + `cli/args.ts:41-44,253-393` (`printHelp`, hand-rolled `parseArgs`, unknown-flag capture) — public.
- Opi entry: `crates/opi-coding-agent/src/cli.rs:112-319` (`#[derive(Parser)]`) + `main.rs:8-39` dispatch — public.
- Comparison: different
- Observable difference: pi accepts unknown `--flags` and routes them to
  extension flag handlers (`args.ts:225-243` `unknownFlags`), prints its own
  help with `--help/-h`; opi is clap-based — unknown long flags are a hard
  parse error, `--help` is clap-rendered (no extension flag sections, no
  environment-variable table, no examples).
- Evidence basis: source inspection
- Review outcome: self-reviewed
- Notes: pi usage text is the flag source of truth for CLI-02…CLI-12.

### CLI-02 — Mode selection: `--mode text|json|rpc`, `--print`, TTY/piped-stdin auto-print
- Dimension(s): 11
- pi entry: `cli/args.ts:66-79` (`--mode`), `:158-165` (`--print/-p`); `main.ts:96-103` (`resolveAppMode`: non-TTY stdin/stdout → print), `:124-127` (`isPlainRuntimeMetadataCommand`), `:73-88` (`readPipedStdin`) — public.
- Opi entry: `cli.rs:150-160` (`--non-interactive`, `--json`, `--rpc` booleans); `main.rs:98-161` (rpc branch, non-interactive branch when `--non-interactive || --json || !prompt.is_empty()`, else interactive) — public.
- Comparison: partial
- Observable difference: no `--mode` flag (three separate booleans; `--json`
  implies non-interactive). `--print` → `--non-interactive` (no `-p` short,
  and a bare positional prompt also triggers one-shot in opi but not pi).
  opi never detects non-TTY stdin/stdout and never reads piped stdin, so
  `echo hi | opi` starts the TUI instead of pi's print mode with prepended
  stdin text. Absence search: `grep -rn "is_terminal\|IsTerminal\|atty" main.rs interactive.rs` → 0 non-test hits; `grep -rn "stdin" main.rs` → only the RPC comment at main.rs:130.
- Evidence basis: source inspection
- Review outcome: self-reviewed
- Notes: `--json-compact` (cli.rs:161-165) is an opi-only modifier (CLI-11).

### CLI-03 — Session selection flags (`--continue/--resume/--session/--session-id/--fork/--session-dir/--no-session/--name`)
- Dimension(s): 11, 13
- pi entry: `cli/args.ts:80-84,93-101,104-110` (flags); `main.ts:347-356` (`resolveSessionPath`: path / exact ID / prefix / global cross-project search), `:365-378` (`validateForkFlags`, `--fork` conflicts), `:380-400` (`validateSessionIdFlags`), `:433-507` (`createSessionManager`: fork prompt-to-fork-global-session, resume picker, `continueRecent`).
- Opi entry: `cli.rs:207-213` (`--resume <ID>`, `--fork <ID>`, `--delete-session <ID>`, `--list-sessions`); `main.rs:80-104,89` dispatch; `session_cli.rs:1020-1112` (`handle_session_cli`) — public.
- Comparison: partial
- Observable difference (flag-by-flag):
  | pi flag | opi | verdict |
  |---|---|---|
  | `--continue, -c` (most recent session for cwd) | none | missing |
  | `--resume, -r` (opens TUI picker) | `--resume <ID>` (exact ID only, stderr status) | different |
  | `--session <path\|id>` (path/partial/global w/ fork offer) | none | missing |
  | `--session-id <id>` (create-or-open fixed ID) | none | missing |
  | `--fork <path\|id>` | `--fork <ID>` (exact ID in sessions dir only) | partial |
  | `--session-dir <dir>` | none (only `OPI_SESSIONS_DIR` env, `session_cli.rs:99-104`) | missing |
  | `--no-session` (in-memory) | none — every run creates a file (`session_coordinator.rs:157-172`) | missing |
  | `--name, -n` | none as flag; `/name` slash command only (`interactive.rs:1987-1995`) | missing |
  | (none) | `--delete-session <ID>`, `--list-sessions [--json]` | opi-only |
  Absence search: `grep -rni '"session-dir|"no_session|"name"|continue' cli.rs main.rs` → only Rust `continue` statements/struct fields; pi-style flag conflicts (`--fork` vs `--session/…`) do not exist.
- Evidence basis: source inspection
- Review outcome: self-reviewed

### CLI-04 — Model/provider flags (`--provider/--model/--models/--api-key/--thinking/--list-models`)
- Dimension(s): 11
- pi entry: `cli/args.ts:85-92,111-116,182-190,233-241`; `main.ts:522-589` (`buildSessionOptions`: `--provider` requires `--model`, `provider/id` + `:<thinking>` shorthand, scoped models for Ctrl+P cycling); `cli/list-models.ts:31-95` (fuzzy search, provider/model/context/max-out/thinking/images columns).
- Opi entry: `cli.rs:118-120` (`--model/-m <spec>` as `provider:model`), `:286-288` (`--list-models [--json]`); `main.rs:1461-1556` (`list_models`: no search arg; PROVIDER/MODEL ID/DISPLAY NAME columns or NDJSON `{model,provider,display_name}`); thinking only via RPC `set_thinking_level` (`rpc.rs:816-833`) and config `thinking_level_map` (`config.rs:1872-1898`).
- Comparison: partial
- Observable difference: no `--provider` (no "requires --model" diagnostic), no
  fuzzy/pattern model resolution, no `:thinking` shorthand, no `--models`
  cycling scope (no Ctrl+P cycling at all — `grep -rn "cycle_model|ctrl_p" .` → 0),
  no `--api-key` runtime override (credentials come from env/config/keyring,
  `provider_factory.rs`), no `--thinking` flag, `--list-models` lacks the
  optional fuzzy search argument and the context/max-out/thinking/images
  columns. Absence search: `grep -rn "api_key\|api-key\|arg.*thinking" cli.rs` → 0 flag hits.
- Evidence basis: source inspection
- Review outcome: self-reviewed

### CLI-05 — Tool flags (`--tools/--exclude-tools/--no-tools/--no-builtin-tools`)
- Dimension(s): 11
- pi entry: `cli/args.ts:131-151` (allowlist w/ `*` patterns, `+name/-name` default mutation, `mcp__` rule; denylist) — help text `args.ts:285-292`.
- Opi entry: `cli.rs:267-277` (`--tools`, `--no-tools`, `--no-builtin-tools`); `policy.rs:118-135` (`resolve_tool_selection`: `--no-tools > --tools > --no-builtin-tools > default`) and `:126-135` (exact-name filter against `BUILTIN_TOOL_NAMES`).
- Comparison: partial
- Observable difference: `--no-tools`/`--no-builtin-tools`/`--tools` exist, but
  opi's allowlist is exact names only (no `*` globs, no `+name/-name` default
  mutation, no `mcp__` handling, no `getToolListError` validation);
  `--exclude-tools`/`-xt` absent (`grep -rn "exclude_tools" .` → 0). opi adds
  `--allow-mutating` gating (CLI-11). Built-in set differs: opi
  `read,write,edit,bash,grep,find,ls,glob` (`policy.rs:20-22`) vs pi
  `read,bash,powershell,edit,write,grep,find,ls` (`args.ts:382-391`) — opi has
  `glob`, lacks `powershell`.
- Evidence basis: source inspection
- Review outcome: self-reviewed
- Notes: tool parity detail is the tools batch's scope; recorded here only as
  flag-observable surface.

### CLI-06 — Resource discovery flags (`--extension/--no-extensions/--no-mcp/--skill/--no-skills/--prompt-template/--no-prompt-templates/--theme/--use-theme/--no-themes/--no-context-files`)
- Dimension(s): 11
- pi entry: `cli/args.ts:168-178,186-232` + resolution in `main.ts:471-475` (`resolveCliPaths`) and `main.ts:516-530` (`resourceLoaderOptions`) — public.
- Opi entry: `cli.rs:112-319` — bounded absence search: `grep -rni "no_extensions|no-extensions|no_mcp|no-mcp|prompt_template|prompt-template|use_theme|use-theme|no_context|no-context|tui_mode|tui-mode" cli.rs main.rs` → 0 hits each; skills/prompt-fragments/themes load only via installed packages (`package_discovery.rs`, `skill.rs`, `prompt_fragment.rs`, `theme_discovery.rs`); context files always discovered, trust-gated (`context_files.rs:1-40`, `harness.rs:1729`), no disable flag.
- Comparison: missing
- Observable difference: no way to load an extension/skill/prompt-template/
  theme from a CLI path, no way to disable extension/skill/theme/context-file
  discovery or MCP for a run. Theme is only `config.defaults.theme`
  (`main.rs:1347`); an unknown theme logs a warning and falls back
  (`interactive.rs:906-909`) — no per-run `--use-theme`.
- Evidence basis: source inspection
- Review outcome: self-reviewed
- Notes: pi's `--no-mcp` is doubly moot in opi: there is no MCP support at all
  (`grep -rln "mcp" opi-coding-agent/src --include="*.rs"` → no files).

### CLI-07 — `--export` session export flag
- Dimension(s): 11, 13
- pi entry: `cli/args.ts:166-167`; `main.ts:313-327` (`exportFromFile`, optional positional output, `Exported to: <path>`, exit 1 on error); renderer `core/export-html/index.ts:273-318` + templates `core/export-html/template.{html,css,js}` + `vendor/` — public.
- Opi entry: `cli.rs:171-205` (`--export-session <ID_OR_PATH>` + `--output` (required) + `--format markdown|json` + `--full-tree` + `--exclude-tool-output` + `--exclude-thinking` + `--redact summary|verbose|none`); `main.rs:56-66,496+` (`run_export_session`); renderer `session_cli.rs:455-700` (markdown/JSON, redaction).
- Comparison: different
- Observable difference: pi exports self-contained themed HTML (base64 session
  payload, marked.js/highlight.js vendors, ANSI-to-HTML tool rendering
  `export-html/ansi-to-html.ts`, `tool-renderer.ts`), default output name
  `<app>-session-<base>.html`. opi writes markdown or re-rendered JSON with
  mandatory `--output`, redaction modes, branch-scope flags; no HTML output
  exists (`grep -rni "html" opi-coding-agent/src` → 0). Exit semantics differ
  (pi prints `Exported to:`; opi requires `--output` else error at
  `main.rs:515`).
- Evidence basis: source inspection
- Review outcome: self-reviewed

### CLI-08 — Project-trust flags (`--approve/-a`, `--no-approve/-na`)
- Dimension(s): 11
- pi entry: `cli/args.ts:217-222` (`projectTrustOverride`); `main.ts:461-463,493-503` (override feeding `resolveProjectTrusted`, trust prompt modes) — public.
- Opi entry: `cli.rs:135-146` (`--trust` / `--no-trust`, mutually exclusive, overriding `[defaults] default_project_trust` and the trust store); `main.rs:201-260,262-320` (`ProjectTrustCli`, two-stage trust-gated config).
- Comparison: different
- Observable difference: same purpose (per-run project-local file trust) but
  different names, no short flags, and opi couples trust with skipping the
  project `.opi/config.toml` layer entirely (`main.rs:250-260`) whereas pi
  gates resource loading through `resolveProjectTrusted` with a TUI
  `createProjectTrustContext` (`cli/project-trust.ts:9-58`). opi's interactive
  trust prompt exists (`TuiTrustChoice`, `main.rs:33-38` doc) but the
  headless behavior differs (ask → Untrusted, `main.rs:170-173`).
- Evidence basis: source inspection
- Review outcome: self-reviewed

### CLI-09 — Input handling & misc flags (`@files`, `--`, multiple messages, `--image`, `--verbose/-v`, `--version/-v`, `--tui-mode`, `--offline`)
- Dimension(s): 11
- pi entry: `cli/args.ts:60-64` (`--`), `:244-251` (`--tui-mode regular|fullscreen`), `:212-215` (`--verbose`), `:216-217` (`--version/-v`), `:223-226` (`--offline`), `:227-228,246-247` (`@file` args); `cli/file-processor.ts:23-70` (text+image files, auto-resize); `cli/initial-message.ts:19-45` (stdin+files+message concat); multiple positional messages sent sequentially (`main.ts:984-986`).
- Opi entry: `cli.rs:316-319` (`prompt: Vec<String>` trailing var-arg, joined with spaces at `main.rs:118`), `:279-281` (`--image <PATH>` repeatable), `:261-263` (`--verbose/-v` debug tracing), clap `version` (`--version`, no `-v`), no `--tui-mode`/`--offline`/`@file` support (`grep -rni "tui_mode|offline|starts_with('@')" cli.rs main.rs` → 0).
- Comparison: different
- Observable difference: pi `-v` prints version, opi `-v` enables debug
  tracing (help-text collision). pi `--verbose` forces verbose startup;
  opi's prints "debug mode" banner only. No `@file` text/image inlining, no
  piped-stdin prepend (CLI-02), no fullscreen/regular TUI switch, no offline
  startup switch (pi also honors `PI_OFFLINE`; opi has no equivalent env —
  `grep -rn "OFFLINE" .` → 0). opi `--image` and `/image` queue images
  (`image.rs`, byte limit `max_image_bytes`) replacing pi's `@image.png`
  auto-resize path. Multiple positional messages become one space-joined
  prompt, not sequential prompts.
- Evidence basis: source inspection
- Review outcome: self-reviewed

### CLI-10 — Top-level subcommands (`install/remove/uninstall/update/list/config/auth/mcp`)
- Dimension(s): 11
- pi entry: `package-manager-cli.ts:285-410` (usage/help + parse: `install <source> [-l]`, `remove`/`uninstall`, `update [source|self|pi] [--self|--extensions|--models|--all|--extension <src>]`, `list`), `:863-948` (`handleConfigCommand` → `config-selector.ts` TUI); `main.ts:228-247` (dispatch incl. `pi mcp` via `extensions/mcp/cli.lazy.ts`); auth subcommand in CLI-19.
- Opi entry: `cli.rs:298-364` (`Command::{Package{Add,Remove,List,Doctor,Enable,Disable}, Doctor}`), `main.rs:31-53` dispatch, `package_cli.rs:69-140` (add: local path or git URL only), `doctor.rs`.
- Comparison: partial
- Observable difference: opi namespaces under `opi package` (no bare
  `install/uninstall` aliases, no `update` at all — no self-update, no
  extension update, no model-catalog refresh; `grep -rn "update" package_cli.rs | head` → only package-update-of-lock semantics), no `config` TUI
  resource selector, no `mcp` command (no MCP). opi adds `package
  doctor/enable/disable` (trust-gated enablement, first enable needs TTY,
  `cli.rs:349-355`) and top-level `doctor` (CLI-11). Sources: pi accepts
  `npm:`, `git:`, `https://`, `ssh://`, local paths; opi accepts local paths
  and git URLs (`package_cli.rs:140-260`).
- Evidence basis: source inspection
- Review outcome: self-reviewed

### CLI-11 — opi-only CLI surface
- Dimension(s): 11
- pi entry: `cli/args.ts:253-393` (bounded absence: none of these flags appear in pi's usage text).
- Opi entry: `cli.rs:120-124` (`--config`), `:150-153` (`--allow-mutating`), `:154-190` (hidden rejected `--sandbox`/`--sandbox-require` with remediation text), `:191-204` (`--execution-backend`/`--execution-strategy`), `:161-165` (`--json-compact`), `:206-213` (`--list-sessions`, `--delete-session`), `:219-227` (`--generate-completion`), `:264-266` (`--trace`), `:300-315` (`Doctor`), package enable/disable.
- Comparison: opi-only
- Observable difference: pi callers have no counterpart; conversely every one
  of these is observable opi behavior (e.g. `--sandbox` always fails with the
  migration remediation, `cli.rs:16-50`).
- Evidence basis: source inspection
- Review outcome: self-reviewed

### CLI-12 — Extension-registered CLI flags (unknown-flag pass-through)
- Dimension(s): 11
- pi entry: `cli/args.ts:225-243` (`unknownFlags` map incl. `--flag=value`), `main.ts:517` (`extensionFlagValues: parsed.unknownFlags`), help rendering `args.ts:259-266` ("Extension CLI Flags"); example `--plan` cited at `args.ts:339`.
- Opi entry: `cli.rs` clap parser rejects unknown flags at parse time; `grep -rn "unknown_flag\|extensionFlag" .` → 0.
- Comparison: missing
- Observable difference: a pi extension can register `--plan`-style flags that
  reach its `parseFlags` hook; an opi invocation with any unlisted long flag
  exits with clap's "unexpected argument" before any extension runs.
- Evidence basis: source inspection
- Review outcome: self-reviewed

### CLI-13 — Interactive mode loop
- Dimension(s): 6, 11
- pi entry: `modes/interactive/interactive-mode.ts` (7211 lines: TUI lifecycle, keybindings, themes, extension widgets/dialogs, `/commands` from `core/slash-commands.ts:20-43`: settings, model, tree, thinking, scoped-models, export, import, share, bug, copy, name, session, changelog, hotkeys, fork, clone, trust, login, logout, new, compact, resume, reload, quit).
- Opi entry: `interactive.rs:895-1622` (`run_interactive_tui` single-screen loop; commands handled at `:1405-1585`: `exit`/`quit` bare words, `/model`, `/name`, `/label`, `/unlabel`, `/session`, `/session info`, `/branch`, `/tree`, `/fork`, `/clone`, `/image`, `/login`, `/logout`, `/help` via auth dispatcher `interactive_auth.rs:162-167`).
- Comparison: partial
- Observable difference: opi covers model/session/branch pickers, fork/clone,
  session metadata, image queue and auth. Missing pi commands: `/settings`,
  `/thinking`, `/scoped-models`, `/export`, `/import`, `/share`, `/bug`,
  `/copy`, `/session` stats view (opi `/session` is a resume picker vs pi's
  info+stats), `/changelog`, `/hotkeys`, `/trust`, `/new`, `/compact`,
  `/resume`-as-command, `/reload`, and slash-`/quit` (opi uses bare
  `quit`/`exit`). No theme switching, extension widgets/dialogs/status-bar
  surfaces in the loop; keybindings limited to submit/abort/new_line
  (`main.rs:1442-1460`).
- Evidence basis: source inspection
- Review outcome: self-reviewed
- Notes: TUI widget/theme/keybinding parity belongs to the interactive/TUI
  batch; only loop-level command surface measured here.

### CLI-14 — Print mode (one-shot text)
- Dimension(s): 6
- pi entry: `modes/print-mode.ts:42-173` — final assistant text to raw stdout, exit 1 on `stopReason error|aborted` or `length` truncation, SIGTERM/SIGHUP handlers killing detached children (129/143), stdin/stdout guard.
- Opi entry: `runner.rs:592-663,705-775` (`run`/`run_with_content`: joins `TextDelta` events, startup diagnostics + bash contract + tool diagnostics to stderr, persist-error suffix); exit codes `runner.rs:47-58` (0/1/2/3/4/130).
- Comparison: different
- Observable difference: opi emits extra stderr prefixes (startup
  diagnostics, execution contracts, tool diagnostics —
  `runner.rs:915-955`); truncation (`length`) does not set a nonzero exit in
  opi; opi distinguishes auth/provider/config failures by exit code 3/4/2
  where pi returns 1; opi has no SIGHUP/SIGTERM child-kill contract visible
  at this layer (cancellation via token).
- Evidence basis: source inspection
- Review outcome: self-reviewed

### CLI-15 — JSON event stream (`--mode json` → opi `--json`)
- Dimension(s): 6
- pi entry: `modes/json-event.ts:44-61` (`toJsonEvent` drops cumulative `partial`, adds `toolcall_start` id/toolName); wire contract `docs/json.md` (flat snake_case `agent_start…agent_settled`, session header = v3 file header first record, delta-only `message_update`, `queue_update`, `entry_appended`, `session_info_changed`, `thinking_level_changed`, `compaction_*`, `auto_retry_*`, `summarization_retry_*`); emission `modes/print-mode.ts:148-151`.
- Opi entry: `runner.rs:99-139,483-610` — first line `{"type":"session_header","schema_version":2}`, then `startup_diagnostics`, then `AgentSessionEvent` envelope `{"type":"Agent","event":{…}}` with CamelCase `AgentEvent` names (`event.rs:29-121`, `session_event.rs:35-141`), terminal `session_summary`; `--json-compact` strips delta snapshots (`runner.rs:99-139`, test at `runner.rs:1109-1166`).
- Comparison: different
- Observable difference: different record vocabulary (opi has no
  `agent_settled`, `entry_appended`, `summarization_retry_*`,
  `bash_execution_update`; opi adds `session_header`, `startup_diagnostics`,
  `session_summary`, `CredentialNeeded/Revoked`), different casing
  (`MessageUpdate` vs `message_update`), nested `event` envelope vs flat
  records, session header carries `schema_version` not the file header, and
  default (non-compact) `message_update` keeps the cumulative message
  snapshot where pi's wire is always delta-only. Framing is LF-NDJSON in both
  (`docs/json.md` vs `rpc.rs:16-19` doc).
- Evidence basis: source inspection
- Review outcome: self-reviewed

### CLI-16 — RPC mode: transport and command set
- Dimension(s): 6, 11
- pi entry: `modes/rpc/rpc-types.ts:17-147` (~40 commands incl. `get_state`, `cycle_model`, `get_available_models`, `cycle_thinking_level`, `get_available_thinking_levels`, `set_steering_mode`, `set_follow_up_mode`, `set_auto_compaction`, `set_auto_retry`, `abort_retry`, `bash`, `abort_bash`, `get_session_stats`, `export_html`, `switch_session`, `fork`, `clone`, `get_fork_messages`, `get_entries`, `get_tree`, `get_last_assistant_text`, `set_session_name`, `get_messages`, `get_commands`, `clear_queue`, `new_session`); transport `modes/rpc/rpc-mode.ts` + `jsonl.ts` (LF JSONL, `response`/event/parse-error records, no ready header — `docs/rpc.md:104-107`).
- Opi entry: `rpc.rs:1-110` (LF JSONL over stdin/stdout, `rpc_ready` header with `schema_version:3`, `mode`, `version`, `startup_diagnostics` at `:495-501`); command set `sdk.rs:58-155`: `prompt, continue, steer, follow_up, abort, set_model, set_thinking_level, compact, session_info, extension_command, trace, quit`; parse-error envelope `rpc.rs:1125-1134,556-557`.
- Comparison: partial
- Observable difference: shared: LF JSONL framing, `response{command,success,id,data|error}` envelope, parse-error record, prompt/steer/follow_up/abort/set_model/set_thinking_level/compact. opi-only: `continue`, `session_info` (rich: model, resources, name, labels, branches, thinking `rpc.rs:856-955`), `extension_command`, `trace`, `quit`, `rpc_ready` header, `error_code` field (`rpc.rs:71-89`). pi-only: everything model/thinking cycling, queue modes, auto-compaction/retry toggles, `bash` RPC execution, session stats, `export_html`, `switch_session`, `fork`/`clone`/`get_entries`/`get_tree`/`get_messages`/`get_last_assistant_text`, `get_commands`, `clear_queue`, `new_session`. opi's prompt response is completion-of-turn semantics (one response per finished run per `rpc.rs:38-46`) vs pi's immediate `disposition` response.
- Evidence basis: source inspection
- Review outcome: self-reviewed
- Notes: both declare unstable 0.x schemas (pi docs vs `sdk.rs:40-43`).

### CLI-17 — RPC extension UI subprotocol
- Dimension(s): 6
- pi entry: `modes/rpc/rpc-types.ts:216-248` (`extension_ui_request` select/confirm/input/editor/notify/setStatus/setWidget/setTitle/set_editor_text + `extension_ui_response`); docs `rpc-extension-ui.md`.
- Opi entry: absence search `grep -rn "extension_ui" rpc.rs sdk.rs` → 0; opi extensions get `extension_command` dispatch only (`rpc.rs:958+`).
- Comparison: missing
- Observable difference: a pi RPC client can render extension dialogs,
  notifications, status-bar and widget surfaces; an opi RPC client cannot —
  extension interaction is limited to one-shot `extension_command` with a
  not-handled error code (`ERR_EXTENSION_COMMAND_NOT_HANDLED`, `rpc.rs:87`).
- Evidence basis: source inspection
- Review outcome: self-reviewed

### CLI-18 — `./rpc-entry` package export (preconfigured RPC main)
- Dimension(s): 6, 11
- pi entry: `rpc-entry.ts:1-13` (sets process title/env, calls `main(["--mode","rpc",…])`); declared public subpath export `package.json:19-21` (`"./rpc-entry": "./dist/bundle/rpc-entry.js"`).
- Opi entry: single bin `opi` (`Cargo.toml:22-24`); lib exposes `pub mod rpc` (`lib.rs:41`) so embedding uses `RpcRunner` directly.
- Comparison: missing
- Observable difference: pi distributions ship a separate importable entry
  that starts RPC mode; opi consumers must run `opi --rpc` or link the
  library — no equivalent preconfigured entry point artifact.
- Evidence basis: source inspection
- Review outcome: self-reviewed
- Notes: pi's `bin` field declares only `pi`; `rpc-entry` is an import-path
  export, not an installed binary.

### CLI-19 — `auth` subcommands (`auth check`, `auth print-api-key`, `auth print-bearer-token`)
- Dimension(s): 11
- pi entry: `cli/auth-command.ts:12-118` (parse/usage/validation incl. `--json`, `--credentials`, `--no-refresh`, `--min-expiry`); execution `main.ts:118-177` (`runAuthCommand`, exit codes 0/1/2); `cli/credential-print.ts`, `cli/auth-check.ts`.
- Opi entry: absence search `grep -rn "print-api-key|print-bearer-token|\"auth\"|auth check" .` → only a stderr string in `main.rs:1515`; `cli.rs` has no `auth` subcommand. Credentials are set via interactive `/login` (CLI-20) or env/config/keyring; nothing prints stored credentials.
- Comparison: missing
- Observable difference: external clients cannot obtain an API key or
  refreshed bearer token from opi on the command line, nor run a
  readiness check with structured exit codes.
- Evidence basis: source inspection
- Review outcome: self-reviewed

### CLI-20 — Interactive `/login` `/logout` and OAuth providers
- Dimension(s): 11
- pi entry: `/login`, `/logout` in `core/slash-commands.ts:37-38` (provider arg, selector UI); login configured per provider via `pi login`-equivalent flows in `extensions/` and `core/auth-storage.ts`, `AuthStorage` (`main.ts:175-176`).
- Opi entry: `interactive_auth.rs:146-230` (`parse_auth_command`, terminal suspend/resume guard, dispatch), `AUTH_HELP` at `:14-20`; providers anthropic, github-copilot, openai-codex (`oauth.rs:40-170` endpoint registry; `github_copilot.rs`, `openai_codex.rs`); non-interactive modes never start login — they emit `CredentialNeeded` with `/login <provider>` remediation (`runner.rs:735-760`, cli.rs after-help `:100-118`).
- Comparison: partial
- Observable difference: opi implements login/logout for exactly three OAuth
  providers with terminal suspension and a one-shot pending-turn retry
  (`interactive.rs:893-899` doc contract); pi's `/login` opens a provider
  selector covering all configured providers (including API-key providers via
  env guidance) and pi additionally exposes non-interactive auth commands
  (CLI-19). Opi has no provider selector: `/login` requires an argument.
- Evidence basis: source inspection
- Review outcome: self-reviewed

### CLI-21 — First-run setup and startup selectors (`startup-ui.ts`)
- Dimension(s): 11
- pi entry: `cli/startup-ui.ts:95-115` (`shouldRunFirstTimeSetup`: official distribution + `PI_EXPERIMENTAL=1` + default agent dir + no settings.json), `:117-137` (`showFirstTimeSetup`: theme preview + analytics opt-in, persists settings), `:139-166` (`showStartupSelector` used for missing-session-cwd and trust prompts), `:196-222` (`showStartupInput`); invoked `main.ts:299-303`.
- Opi entry: absence search `grep -rni "first_time|first-run|FirstRun" .` → only unrelated strings (`doctor.rs:662`, `evidence.rs:684`); no startup TUI, no theme/analytics onboarding, no generic selector/input components at startup (trust prompt is a bespoke `TuiTrustChoice`, `main.rs:33-38`).
- Comparison: missing
- Observable difference: first opi run gives no onboarding; a resumed session
  whose stored cwd is missing proceeds with the stored cwd as
  `workspace_root` (`main.rs:98-99,1179-1183`) instead of pi's interactive
  Continue/Cancel selector or non-interactive `MissingSessionCwdError` exit
  (`core/session-cwd.ts:23-50`, `main.ts:322-337`).
- Evidence basis: source inspection
- Review outcome: self-reviewed
- Layer H: pi first-time setup is explicitly **experimental** — gated behind
  `PI_EXPERIMENTAL=1` and official-distribution checks, code comment
  `startup-ui.ts:123-130`; authority wording: none beyond the gate
  (implementation-level); observed maturity: implemented, experimental.
  Implemented experimental behavior also counted as Layer A with exposure
  "interactive first run only".

### CLI-22 — Startup migrations and deprecation warnings
- Dimension(s): 11, 13
- pi entry: `migrations.ts:21-330` (`migrateAuthToAuthJson` oauth.json/settings.json apiKeys→auth.json; `migrateSessionsFromAgentRoot`; commands→prompts; keybindings config; tools→bin; deprecated extension dirs; extension-system migration; `showDeprecationWarnings` interactive dialog; `runMigrations` at `main.ts:282-284`); session v1→v2→v3 in `core/session-manager.ts:289-347`.
- Opi entry: absence search `grep -rni "migrat" main.rs config.rs session_cli.rs` → only session-format v1 read-only acceptance (`session.rs:46-55` `SESSION_FORMAT_POLICY`: "version 1 is read-only legacy input; opi sessions are not pi-session-v3 compatible"); no config/auth/location migrations run at startup.
- Comparison: missing
- Observable difference: opi performs no one-time upgrades of legacy layout
  or credentials on startup; its only version handling is rejecting v1 for
  writes and accepting it read-only (fork of a v1 session fails closed,
  `session_cli.rs:196-201`).
- Evidence basis: source inspection
- Review outcome: self-reviewed
- Layer H: pi `docs/session-format.md:29-36` documents v1→v3 auto-migration as
  normative product behavior.

### CLI-23 — Self-update / package update pipeline (`pi update`, managed install)
- Dimension(s): 11
- pi entry: `package-manager-cli.ts:363-387,1092+` (`update [source|self|pi] [--self|--extensions|--models|--all] [--force]`, `PI_SELF_UPDATE_COMMAND` template, Windows self-update quarantine cleanup `utils/windows-self-update.ts` invoked `main.ts:199-202`, `cleanupManagedInstall` `main.ts:205`).
- Opi entry: absence search `grep -rn "self.update\|self_update\|update" package_cli.rs cli.rs` → no update subcommand; packages update only implicitly on `add` re-run (lock refresh `package_cli.rs:266-330`).
- Comparison: missing
- Observable difference: `opi update` does not exist; no self-update command,
  no extension/model-catalog refresh command, no managed-install cleanup.
- Evidence basis: source inspection
- Review outcome: self-reviewed

### SES-01 — Session storage layout and file naming
- Dimension(s): 13
- pi entry: `core/session-manager.ts:586-603` (default dir `~/.pi/agent/sessions/--<encoded-cwd>--/`), `:1062-1083` (file `<ISO-ts-with-:-→->_<uuidv7>.jsonl`), ID `createSessionId` = uuidv7 `:264-266`; entry IDs 8-hex collision-checked `:276-283`; `docs/session-format.md:10-16`.
- Opi entry: `session_cli.rs:99-118` (`session_dir()`: `OPI_SESSIONS_DIR` env → `~/.local/share/opi/sessions/` (Unix) / `%LOCALAPPDATA%\opi\sessions\` (Windows), flat, no per-cwd subdir); file name `{id}.jsonl` with id = lowercase hex milliseconds (`session_coordinator.rs:171`, `session_cli.rs:1130-1137`); entry ids `msg-N`/`cmp-N`/`sum-N`/`leaf-N` from a process-global counter (`session_coordinator.rs:22,365,463,485,588`).
- Comparison: different
- Observable difference: different root, flat vs cwd-encoded per-directory
  layout, different file names (no timestamp prefix; ids are millisecond hex
  so two sessions in one ms collide — fork retries with `-N` suffixes,
  `session_cli.rs:1077-1108`), different ID/entry-ID grammars. Opi session
  lookup is therefore global-by-id while pi groups sessions per project
  directory.
- Evidence basis: source inspection
- Review outcome: self-reviewed

### SES-02 — Session format version and compatibility policy
- Dimension(s): 13
- pi entry: `CURRENT_SESSION_VERSION = 3` (`core/session-manager.ts:41`); header `{type:"session",version,id,timestamp,cwd,parentSession?}` `:43-51`; auto-migration v1→v2→v3 on load `:289-347` with file rewrite; `docs/session-format.md:29-36`.
- Opi entry: `session.rs:46-55` — `FORMAT_VERSION = 2`, `LEGACY_FORMAT_VERSION = 1` (read-only), `SESSION_FORMAT_POLICY` explicitly "opi sessions are not pi-session-v3 compatible… Unknown required v2 entries fail closed"; header adds required `runtime_input_binding` (`session.rs:57-91`, writer rejects non-v2 `session.rs:466-472`).
- Comparison: different
- Observable difference: opi files carry a `runtime_input_binding` field and
  version 2; pi files cannot be opened by opi and vice versa (pi would treat
  an opi file as unknown-version/foreign; opi fails closed on required unknown
  entries). Deliberate incompatibility is still `different` under the strict
  reproduction lens.
- Evidence basis: source inspection
- Review outcome: self-reviewed

### SES-03 — Fork / branch semantics
- Dimension(s): 13
- pi entry: `SessionManager.forkFrom` `core/session-manager.ts:1826-1882` (copies full history into new session under target cwd, header `parentSession = source path`, fresh id); in-session `branch()`/`moveToEntry` tree navigation `:1528-1610`; RPC `fork {entryId}` forks at an entry.
- Opi entry: `session_cli.rs:196-330` (`fork_session*`: copies every active-chain content+metadata entry, writes fresh `Leaf` at active tip, `parent_session = source id`, collision-retry target allocation); interactive `/fork`/`/clone` via `harness.fork_current_session` (`interactive.rs:1500-1517`); branch switch `/branch`,`/tree` → `resume_session_branch_tip` (`interactive.rs:1326-1347`); RPC-level entry-point fork absent (no `fork` command).
- Comparison: partial
- Observable difference: CLI-level fork exists on both (pi resolves
  path/partial/global ids and prompts; opi takes exact ids only — CLI-03).
  In-session fork-at-entry (`/fork` from a previous user message, RPC
  `fork {entryId}`) is absent in opi: opi's `/fork`/`/clone` always fork at
  the active tip. Opi persists an explicit `Leaf` pointer entry; pi tracks the
  leaf in memory (last file-order entry) and rewrites only on migration.
- Evidence basis: source inspection
- Review outcome: self-reviewed

### SES-04 — Continue / resume / discovery semantics
- Dimension(s): 13
- pi entry: `SessionManager.continueRecent` `:1804-1812` (most recent in dir, cwd-filtered when custom dir), `findById` (header-only exact scan) `:1890-1911`, `list`/`listAll` with progress+abort `:1916-1931`, `--resume` TUI picker with Tab all-projects toggle (`cli/session-picker.ts:16-52`), missing-cwd prompt/error (`core/session-cwd.ts`, `main.ts:322-337`).
- Opi entry: `session_cli.rs:141-193` (`list_sessions` reads every file fully, reconstructs active-branch metadata, sorts by timestamp desc), `:169-193` (`resume_session` exact-id file), `:905-912` `--list-sessions` output (text or `--json`), `/session` in-TUI overlay picker (`picker.rs:60-101`, `interactive.rs:1450-1470`); no continue-recent, no partial/global id search, no progress/abort on listing.
- Comparison: different
- Observable difference: `opi --resume <id>` requires the exact id; no
  continue-most-recent; pickers list only the flat sessions dir (all projects
  mixed) with no cross-project toggle; resume prints recovery warnings
  (`session_cli.rs:79-91`) where pi loads silently (migrations aside); the
  resumed session's stored cwd becomes workspace_root unconditionally
  (CLI-21).
- Evidence basis: source inspection
- Review outcome: self-reviewed

### SES-05 — Session name / label / metadata entries
- Dimension(s): 13
- pi entry: `session_info` entry `core/session-manager.ts:141-145` (name), `label` entries `:130-139` (set/unset with timestamps), `--name` CLI flag `main.ts:339-348`, `sessionName` lookup `:1180-1190`.
- Opi entry: `session_coordinator.rs` `name`/`labels` fields seeded on open (`:150-160` doc), `append_session_info`/`add_label`/`remove_label` via harness (`interactive.rs:2042-2059`); `/name`,`/label`,`/unlabel` commands (`interactive.rs:1987-2023`); surfaced in `--list-sessions --json`, session picker and RPC `session_info` (`picker.rs:60-101`, `rpc.rs:888-899`).
- Comparison: partial
- Observable difference: entry-level parity exists (latest-wins session_info,
  add/remove labels), but opi has no `--name` startup flag (name settable
  only mid-session) and no `session_info_changed`-equivalent JSON event
  shape (opi's `SessionInfoChanged` carries an extra `session_id` field,
  `session_event.rs:78-81`; pi's carries only `name`, `docs/json.md`).
- Evidence basis: source inspection
- Review outcome: self-reviewed

### SES-06 — Persistence lifecycle (lazy creation, in-memory sessions)
- Dimension(s): 13
- pi entry: `core/session-manager.ts:1166-1180` (`_hasConversation`: file created only on first user/assistant message; setup entries stay in memory), `_persist` `:1193-1213` (`wx` exclusive create then append), `inMemory` `:1815-1817` for `--no-session`/help/list-models.
- Opi entry: `session_coordinator.rs:122-172` (`SessionCoordinator::new` → `SessionWriter::create` writes the header file immediately, `create_dir_all` included); every run mode creates a durable session; no in-memory/no-session mode (absence: CLI-03).
- Comparison: different
- Observable difference: opening and closing opi without chatting leaves an
  empty session file behind and pollutes `--list-sessions`; pi leaves no
  file. `opi --list-models`/`doctor`/export exit before session creation
  (`main.rs:44-79`) but any actual run persists immediately.
- Evidence basis: source inspection
- Review outcome: self-reviewed

### SES-07 — HTML export (`export-html/`, `session-export.ts`)
- Dimension(s): 13
- pi entry: `core/export-html/index.ts:186-318` (`exportSessionToHtml` for `/export`, `exportFromFile` for `--export`), `ansi-to-html.ts` (258 L), `tool-renderer.ts` (177 L), templates `template.html/css/js` + vendored marked/highlight; slash `/export` (`core/slash-commands.ts:25`) defaults HTML or `.jsonl` by extension; RPC `export_html {outputPath?}`.
- Opi entry: absence searches: `grep -rni "export.*html|html.*export" --include="*.rs" crates/` → only `opi-ai/src/bedrock/sigv4.rs`, `opi-ai/src/retry.rs` (unrelated); `grep -rni "html" opi-coding-agent/src` → 0; no `/export` command in `interactive.rs` dispatch (`:1405-1585`); no RPC `export_html` (`sdk.rs:58-155`).
- Comparison: missing
- Observable difference: no code path in opi renders a session to HTML via
  CLI, slash command, or RPC; export is markdown/JSON only (CLI-07/SES-08).
- Evidence basis: source inspection
- Review outcome: self-reviewed

### SES-08 — JSONL session serialization export
- Dimension(s): 13
- pi entry: `core/session-export.ts:8-46` (`serializeSessionBranch` re-serializes the active branch with a fresh header and linearized parentIds; `exportSessionToJsonl` default `session-<ts>.jsonl` in cwd) used by `/export <path>.jsonl` and share flows.
- Opi entry: `session_cli.rs:455-543` (`export_session`: `--format json` renders `{session:{id,timestamp,cwd,parent_session},messages:[…]}` with per-block redaction, `--redact` default summary, optional `--full-tree`; markdown renderer `:545-650`); no raw re-serialization mode.
- Comparison: different
- Observable difference: opi's JSON export is a lossy re-render (thinking
  omitted unless `--include-thinking`, signatures always dropped, redaction
  applied, tool output excludable) rather than pi's byte-faithful branch
  re-serialization with fresh linearized ids; opi requires `--output`, pi
  derives one.
- Evidence basis: source inspection
- Review outcome: self-reviewed

### SES-09 — Session picker surfaces
- Dimension(s): 13, 6
- pi entry: `cli/session-picker.ts:16-52` (standalone startup TUI with incremental progress, cancel exits 0, Tab all-projects toggle via two loaders).
- Opi entry: `picker.rs:60-101` (`session_picker_items` for the in-TUI overlay), `interactive.rs:1450-1470` (`/session` opens `PickerKind::Session` overlay, resume via `harness.resume_session_id`).
- Comparison: partial
- Observable difference: pi's picker runs before the agent starts (with
  progress callbacks and abort signal); opi's picker exists only inside a
  running TUI session, lists the flat dir without progress, and offers no
  all-projects toggle (single source list).
- Evidence basis: source inspection
- Review outcome: self-reviewed

---

## Layer H — active design material encountered in scope

| Claim | Anchor | Authority wording | Observed maturity |
|---|---|---|---|
| First-time setup (theme + analytics opt-in) is experimental, official-dist-only, `PI_EXPERIMENTAL=1`-gated | `cli/startup-ui.ts:123-137`; `main.ts:296-299` "Experimental first-time setup" | none beyond the code gate | implemented, experimental |
| JSON event stream page is "the canonical reference for events shared by JSON and RPC mode" | `docs/json.md:7` | normative ("canonical reference") | implemented |
| RPC protocol docs (framing, correlation, lifecycle, errors) | `docs/rpc.md:14-124` | normative reference pages | implemented |
| Session file format v3 with auto-migration, per-cwd storage, trash-based delete | `docs/session-format.md:1-36` | normative | implemented |
| RPC extension UI subprotocol (dialogs/notifications/widgets over RPC) | `docs/rpc-extension-ui.md`; `rpc-types.ts:216-248` | normative subprotocol | implemented |
| CLI flags reference incl. extension-registered flags and env-var table | `docs/cli.md`; `args.ts:253-393` | normative ("`pi --help` is authoritative for the installed version") | implemented |
| `--tui-mode regular|fullscreen` dual TUI geometry | `args.ts:244-251`; `docs/tui.md` | normative flag | implemented |
| Opi session format policy: v2 required, v1 read-only, not pi-v3 compatible, fail-closed unknown entries | `crates/opi-agent/src/session.rs:55` (`SESSION_FORMAT_POLICY`, test-asserted) | normative constant | implemented |
| Opi RPC/SDK protocol unstable 0.x, schema_version 3, clients must check | `rpc.rs:10-14`; `sdk.rs:38-43` | normative doc comment | implemented |

No TODO/FIXME markers with design content were found in the batch's pi scope
(`grep -rn "TODO\|FIXME\|XXX" cli*.ts cli/ core/session-manager.ts core/session-export.ts core/session-cwd.ts core/export-html/ modes/*.ts modes/rpc/ migrations.ts rpc-entry.ts` → 0).

## Coverage honesty

Not inspected, and why:

- **pi interactive TUI internals** (`modes/interactive/interactive-mode.ts`
  beyond the command/event dispatch excerpts, `components/`, `theme/`,
  keybindings, `session-share.ts`, `bug-report.ts`, external editor): the
  interactive/TUI batch owns widget/theme/keybinding parity; this batch
  measured only the loop's command surface (CLI-13) and startup theme hooks.
- **pi `core/sdk.ts`, `agent-session*.ts`, `package-manager.ts` internals**:
  embedding/API seam, other batches. Only their CLI-relevant entry points
  were read.
- **pi `mcp` subcommand implementations** (`extensions/mcp/cli.ts`): verified
  absence of any MCP surface in opi (file-level grep) but did not enumerate
  pi's `mcp add/remove/list/login/logout` behaviors; belongs to the
  extensions/MCP batch.
- **opi `harness.rs` (8132 L) session/branch internals**: read only the
  builder paths cited (resume, fork_on_start, context files); branch-tree
  reconstruction parity is measured via `session_branch.rs` outputs referenced
  from `picker.rs`/`rpc.rs`.
- **pi `extensions/` built-ins** (codemode, llama, tool-search): not CLI
  surface beyond the flags counted in CLI-06.
- No probes executed (read-only audit); all verdicts are source-inspection
  based, per the independence constraints.
