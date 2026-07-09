//! Shared test-support helpers for `opi-coding-agent` integration tests.
//!
//! Each `tests/*.rs` binary pulls this in via `mod common;` using Cargo's
//! standard `tests/common/mod.rs` pattern: files in subdirectories of `tests/`
//! are NOT compiled as separate test binaries, only included as modules. This
//! module is compiled once per binary that includes it and never participates
//! in the published crate surface — `opi-coding-agent` exposes its library
//! through `src/`, not `tests/`, so `tests/common` cannot leak into the
//! crates.io API.
//!
//! Bodies here are kept byte-identical to the per-binary copies they replace.

// Each test binary compiles this module independently, so a helper used by some
// binaries and not others (e.g. `create_gitignore` is unused by `ls_tool`) is
// expected to be dead code in those binaries. Suppress per-binary dead_code
// rather than forcing every binary to touch every helper.
#![allow(dead_code)]

use opi_agent::tool::ToolResult;

/// Concatenate every `OutputContent::Text` fragment of a tool result into one
/// string. Byte-identical to the per-binary copies it replaces.
pub fn tool_result_text(result: &ToolResult) -> String {
    result
        .content
        .iter()
        .filter_map(|c| match c {
            opi_ai::message::OutputContent::Text { text } => Some(text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("")
}

/// Write a `.gitignore` file containing `content` into `dir`.
pub fn create_gitignore(dir: &std::path::Path, content: &str) {
    std::fs::write(dir.join(".gitignore"), content).unwrap();
}
