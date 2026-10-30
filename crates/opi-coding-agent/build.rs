//! Build configuration for the `opi` binary (task 20.34).
//!
//! The non-interactive turn pipeline drives a deep async future chain on the
//! Windows main thread. The MSVC linker's default stack reserve is 1 MiB,
//! which the pipeline already ran close to (the `Box::pin(run_non_interactive(
//! ..))` site in `main.rs` exists for exactly that); once the config/provider
//! construction layers grew, unoptimized builds overflowed the main-thread
//! stack (`0xC00000FD`) mid-turn. Give the package's binaries an explicit
//! 32 MiB reserve so debug builds keep working headroom.

fn main() {
    if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc") {
        println!("cargo:rustc-link-arg-bins=/STACK:33554432");
    }
    println!("cargo:rerun-if-changed=build.rs");
}
