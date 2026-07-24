# opi-tui

[![Crates.io](https://img.shields.io/crates/v/opi-tui.svg)](https://crates.io/crates/opi-tui)
[![Docs.rs](https://docs.rs/opi-tui/badge.svg)](https://docs.rs/opi-tui)

> Ratatui-based terminal UI widgets used by [opi](https://github.com/OdradekAI/opi).

[Simplified Chinese](README.zh.md) | [opi workspace](../../README.md)

Ratatui-based terminal UI widget library for Rust: transcript, diff and
markdown rendering, fuzzy pickers, terminal-image escapes, themes, and
keybindings — the rendering layer `opi-coding-agent`'s TUI uses.

```sh
cargo add opi-tui
```

Requires Rust 1.97+ (workspace MSRV; edition 2024) and ratatui 0.30 / crossterm 0.29.

## Status

Current crate version: `0.7.1`, inherited from the workspace package version.

`opi-tui` is a synchronous widget library. Callers own the event loop, async
runtime, terminal setup, and application state. The crate provides rendering
primitives used by `opi-coding-agent`'s interactive TUI.

It does not call providers, run tools, read sessions, load packages, or manage
background tasks. Those responsibilities stay in `opi-agent` and
`opi-coding-agent`. Tool execution, diagnostics, provider correctness, retries,
usage, session storage, and session export stay outside this crate; `opi-tui`
renders the resulting transcript, tool status, image placeholders/escapes,
picker rows, and edit diff previews.

## Components

| Item | Purpose |
|------|---------|
| `Shell` | Top-level layout for transcript, status bar, editor, and optional tool-call view. |
| `MessageList` | Scrollable conversation transcript with role styling, diffs, and image payloads. |
| `InputEditor` | Multi-line input buffer with cursor/edit helpers. |
| `StatusBar` | App state, model, and token/cost status. |
| `ToolCallView` | Tool-call line with name, args, and status. |
| `MarkdownView` / `CodeBlock` | Markdown and fenced code-block rendering. |
| `DiffView` | Unified diff rendering for before/after file edits and edit-preview snapshots. |
| `SelectList` / `SelectListState` | Fuzzy-select list for model, session, and tree pickers. |
| `BranchPicker` / `BranchPickerState` | Session branch picker with active-branch marking and Unicode-width-aware rows. |
| `terminal_image` | Kitty/iTerm2/Sixel escape helpers plus text fallback. |
| `Theme` / `resolve_theme` | Semantic palettes; built-in `default` and `monokai`. |
| `Keybindings` / `KeyCombo` | Configurable semantic actions: submit, abort, and new line. |

## Terminal Images

`terminal_image` exposes:

- `TerminalGraphicsProtocol::{Kitty, Iterm2, Sixel, Fallback}`
- `detect_graphics_protocol`
- `kitty_escape`, `iterm_escape`, `sixel_escape`, and `text_fallback`
- `ImageData` for PNG, JPEG, GIF, and WebP metadata

Protocol detection recognizes Kitty and iTerm2 from environment hints and falls
back to text placeholders otherwise. `sixel_escape` is public but currently
returns an empty string, so callers should treat Sixel output as not implemented
until that function emits encoded content.

## Keybindings and Themes

Default keybindings:

| Action | Default |
|--------|---------|
| submit | `enter` |
| abort | `escape` |
| new line | `alt+enter` |

`KeyCombo` parses lowercase strings such as `enter`, `escape`, `ctrl+c`,
`alt+enter`, and `shift+tab`. Invalid config is handled by the caller; the
`opi` binary falls back to defaults.

`Theme` exposes semantic color fields for roles, status bar, editor, markdown,
code blocks, diffs, and tool status. `resolve_theme(name)` recognizes `default`
and `monokai`; unknown names resolve to `default`.

For custom themes, `parse_color`, `THEME_TOKENS`, `is_valid_token`, and
`Theme::from_color_map` form the theme-discovery API. These types are an
**unstable 0.x extension API** and may break between minor versions.

## Integration Pattern

The `opi` binary uses this crate from `crates/opi-coding-agent/src/interactive.rs`:

1. Keep application state in the caller.
2. Update state from `opi_agent::AgentEvent` callbacks.
3. Resolve `Theme` and `Keybindings`.
4. Build picker overlays when needed.
5. Build a `Shell` each frame and render it with ratatui.

## Public Modules

`branch_picker`, `diff_view`, `editor`, `keybindings`, `markdown`,
`message_list`, `render`, `select_list`, `status_bar`, `terminal_image`,
`theme`, and `tool_call`.

## License

MIT. See the workspace [LICENSE](../../LICENSE).
