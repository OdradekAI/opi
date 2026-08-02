//! The `opi-sandbox` standalone binary entry point (Phase 16 task 16.11.2).
//!
//! It reads `argv`, dispatches through the library [`cli`] module, and exits with
//! the mapped code. The binary is dependency-neutral (no `opi` access, no durable
//! state); the standalone smoke suite proves that in isolation. All CLI logic
//! lives in [`opi_sandbox::cli`] so it is exercised by the portable contract
//! tests; this entry is intentionally a one-liner wrapper.

#[tokio::main]
async fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().collect();
    let code = opi_sandbox::cli::run(args).await;
    // Every CLI code (target exits 0-255, the reserved 2/124/125/130, and the
    // Unix 128+signal mapping up to 159) fits a byte; truncation matches the OS
    // exit-code convention.
    std::process::ExitCode::from(code as u8)
}
