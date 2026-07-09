# opi-tui

[![Crates.io](https://img.shields.io/crates/v/opi-tui.svg)](https://crates.io/crates/opi-tui)
[![Docs.rs](https://docs.rs/opi-tui/badge.svg)](https://docs.rs/opi-tui)

> [opi](https://github.com/OdradekAI/opi) 使用的 ratatui 终端 UI 组件库。

[English](README.md) | [opi workspace](../../README.zh.md)

基于 ratatui 的 Rust 终端 UI 组件库：transcript、diff 与 markdown 渲染、模糊选择器、
terminal-image escape、主题和按键绑定——`opi-coding-agent` TUI 使用的渲染层。

```sh
cargo add opi-tui
```

需要 Rust 1.85+（edition 2024）以及 ratatui 0.30 / crossterm 0.29。

## 当前状态

当前 crate 版本是 `0.7.0`，继承自 workspace 包版本。

`opi-tui` 是同步 widget library。调用方负责事件循环、异步 runtime、终端初始化和
应用状态。本 crate 提供 `opi-coding-agent` 交互式 TUI 使用的渲染基础组件。

它不调用 Provider、不运行工具、不读取会话、不加载 package，也不管理后台任务。
这些职责分别留在 `opi-agent` 和 `opi-coding-agent`。工具执行、diagnostics、provider
正确性、retry、usage、session 存储和 session 导出都留在本 crate 之外；`opi-tui` 负责
渲染由上层传入的 transcript、工具状态、图片占位/escape、选择器行和编辑 diff 预览。

## 组件

| 项 | 作用 |
|----|------|
| `Shell` | 对话记录、状态栏、编辑器和可选工具调用视图的顶层布局。 |
| `MessageList` | 可滚动对话记录，支持角色样式、diff 和图片 payload。 |
| `InputEditor` | 多行输入缓冲区，带光标和编辑辅助。 |
| `StatusBar` | 应用状态、模型和 token/费用状态。 |
| `ToolCallView` | 展示工具名、参数和状态的工具调用行。 |
| `MarkdownView` / `CodeBlock` | Markdown 与 fenced code block 渲染。 |
| `DiffView` | 为文件编辑 before/after 和编辑预览快照渲染 unified diff。 |
| `SelectList` / `SelectListState` | 模型、会话和会话树选择器使用的 fuzzy-select 列表。 |
| `BranchPicker` / `BranchPickerState` | 会话分支选择器，支持活跃分支标记和 Unicode 宽度感知行。 |
| `terminal_image` | Kitty/iTerm2/Sixel escape 辅助与文本 fallback。 |
| `Theme` / `resolve_theme` | 语义调色板；内置 `default` 和 `monokai`。 |
| `Keybindings` / `KeyCombo` | 可配置语义动作：submit、abort、new line。 |

## 终端图片

`terminal_image` 暴露：

- `TerminalGraphicsProtocol::{Kitty, Iterm2, Sixel, Fallback}`
- `detect_graphics_protocol`
- `kitty_escape`、`iterm_escape`、`sixel_escape` 和 `text_fallback`
- 带 PNG、JPEG、GIF、WebP 元数据的 `ImageData`

协议检测会根据环境线索识别 Kitty 和 iTerm2，其他情况回退为文本占位符。
`sixel_escape` 是公开函数，但当前返回空字符串；在该函数真正输出编码内容前，调用方
应把 Sixel 输出视为未实现。

## 按键绑定与主题

默认按键：

| 动作 | 默认值 |
|------|--------|
| submit | `enter` |
| abort | `escape` |
| new line | `alt+enter` |

`KeyCombo` 解析小写字符串，例如 `enter`、`escape`、`ctrl+c`、`alt+enter` 和
`shift+tab`。无效配置由调用方处理；`opi` 二进制会回退到默认值。

`Theme` 为角色、状态栏、编辑器、Markdown、代码块、diff 和工具状态提供语义颜色。
`resolve_theme(name)` 识别 `default` 和 `monokai`；未知名称解析为 `default`。

自定义主题可通过 `parse_color`、`THEME_TOKENS`、`is_valid_token` 和
`Theme::from_color_map` 组成的主题发现 API 加载。这些类型属于**不稳定的 0.x 扩展
API**，可能在次版本间发生破坏性变更。

## 集成模式

`opi` 二进制在 `crates/opi-coding-agent/src/interactive.rs` 中使用本 crate：

1. 调用方持有应用状态。
2. 根据 `opi_agent::AgentEvent` 回调更新状态。
3. 解析 `Theme` 和 `Keybindings`。
4. 在需要时构建选择器 overlay。
5. 每帧构建一个 `Shell`，并通过 ratatui 渲染。

## 公共模块

`branch_picker`、`diff_view`、`editor`、`keybindings`、`markdown`、
`message_list`、`render`、`select_list`、`status_bar`、`terminal_image`、
`theme` 和 `tool_call`。

## 许可证

MIT。详见 workspace [LICENSE](../../LICENSE)。
