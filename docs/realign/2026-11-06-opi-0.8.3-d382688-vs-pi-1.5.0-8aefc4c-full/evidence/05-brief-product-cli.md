# Batch CLI/SES — CLI, modes, sessions, export (Dims 11,6,13)

pi side: `packages/coding-agent/src/{cli.ts, main.ts, cli/ (args.ts, setup.ts,
auth-command.ts, auth-check.ts, credential-print.ts, list-models.ts,
session-picker.ts, project-trust.ts, initial-message.ts, file-processor.ts,
config-selector.ts, startup-ui.ts), modes/ (index.ts, interactive/,
print-mode.ts, json-event.ts, rpc/), core/{session-manager.ts, session-cwd.ts,
session-export.ts, export-html/, migrations.ts}, rpc-entry.ts,
package-manager-cli.ts}` + docs `cli.md, rpc.md, rpc-commands.md, json.md`.

Opi side: `crates/opi-coding-agent/src/{cli.rs, main.rs, runner.rs,
runner/json_output.rs, interactive.rs, interactive_auth.rs, rpc.rs,
session_cli.rs, session_coordinator.rs, picker.rs, model_listing.rs, image.rs,
config.rs, oauth.rs}`.

Measure:
1. CLI flag parity table: every pi flag (args.ts usage text) vs opi flags
   (cli.rs): --mode text/json/rpc, --print, --continue/--resume/--session/
   --fork/--session-id/--session-dir/--no-session/--name, --models cycling,
   --tools/--exclude-tools/--no-tools/--no-builtin-tools, --thinking levels,
   --extension/--no-extensions, --skill/--no-skills, --prompt-template,
   --theme/--use-theme/--no-themes, --no-context-files, --export,
   --list-models, --tui-mode, --approve/--no-approve, --offline, --image,
   --append-system-prompt, --provider/--model/--api-key, --system-prompt,
   --verbose. Flag-by-flag verdict.
2. Modes: interactive loop vs `interactive.rs`; print mode; json-event stream
   format (event names/payloads/NDJSON framing) vs `runner/json_output.rs`;
   RPC: transport + full command set (docs/rpc-commands.md + modes/rpc source)
   vs `rpc.rs`.
3. Session storage: file layout, naming, version fields, fork/branch/continue/
   resume semantics vs `session_coordinator.rs`/`session_cli.rs`.
4. HTML export (`export-html/`, `session-export.ts`) vs opi (absence search).
5. First-run setup/startup-ui/auth commands vs `interactive_auth.rs`/`oauth.rs`.

Output `evidence/product-cli.md`, prefixes `CLI`, `SES`. Tallies + coverage
honesty; exact absence-search commands.
