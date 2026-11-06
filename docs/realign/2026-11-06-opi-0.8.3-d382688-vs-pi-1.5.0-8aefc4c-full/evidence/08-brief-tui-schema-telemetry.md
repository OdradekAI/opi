# Batch TUI/SCM/TEL — terminal UI, schema library, telemetry (Dims 9,19,17)

(A) pi `packages/tui` (src/index.ts exports; renderer, components, regular vs
fullscreen modes, keybindings, IME, mouse, themes, images, terminal restore)
vs opi `crates/opi-tui/src/*` (render.rs, markdown.rs, message_list.rs,
status_bar.rs, tool_call.rs, editor.rs, select_list.rs, keybindings.rs,
theme.rs, terminal_image.rs, diff_view.rs, permission_prompt.rs,
trust_prompt.rs, branch_picker.rs). Enumerate pi public components from
index.ts; map each to opi counterpart or bounded absence; note fullscreen vs
regular mode coverage; note opi-only components.

(B) pi `packages/schema` (JSON Schema validation, drafts 2020-12 + 7,
compiled closures, builders obj/str/int/opt/oneOf, JSON Pointer errors,
2222/2223 suite claim) vs opi `crates/opi-ai/src/schema/` — draft coverage,
builder API, error format, no-eval property vs Rust.

(C) pi `packages/telemetry` (TelemetryContext/Span callback contract, NOOP,
propagation, counters/histograms, InMemory reference, typed schemas) vs opi
`crates/opi-ai/src/telemetry.rs` (+ `rg -li telemetry
crates/opi-coding-agent/src`) — contract shape, span propagation, metrics,
exporters.

Output `evidence/tui-schema-telemetry.md`, prefixes `TUI`, `SCM`, `TEL`.
Tallies + coverage honesty; exact absence-search commands.
