#!/usr/bin/env bash
set -euo pipefail

# opi-implement smoke — parameterized verify gates.
#
# Modes (first argument, defaults to `boot`):
#   boot    A.3 boot ritual: workspace builds and lints clean. No tests, no test-binary
#           compilation (uses --lib, never --all-targets, to avoid compiling every test
#           binary in the workspace — the host disk constraint).
#   full    workspace-tier D.3: boot gates PLUS clippy --all-targets, rustdoc, and the
#           full workspace test. Reserved for cross-crate / workspace-tier tasks.
#   scoped --crate <c> [--test <t> ...]
#           non-workspace D.3 (or an explicit per-crate re-check): build --workspace
#           (cross-crate compile safety) plus scoped clippy/test for ONE crate. Pass an
#           explicit --test <name> per relevant test binary; with no --test only the
#           crate lib test runs. Never compiles the whole workspace's test binaries.
#
# CARGO_TARGET_DIR is honored from the environment. The opi-implement skill sets a
# per-session directory off the repository drive (e.g. E:\opi-target\<session>) before
# invoking this script.

mode="${1:-boot}"
shift || true

echo "=== opi-impl smoke [$mode] ==="

rustc --version >/dev/null 2>&1 || { echo "FAIL: rustc not found"; exit 1; }
cargo --version >/dev/null 2>&1 || { echo "FAIL: cargo not found"; exit 1; }

case "$mode" in
  boot)
    echo "Checking workspace build..."
    cargo build --workspace 2>&1 || { echo "FAIL: cargo build --workspace"; exit 1; }
    echo "Checking format..."
    cargo fmt --check --all 2>&1 || { echo "FAIL: cargo fmt --check"; exit 1; }
    echo "Checking clippy (lib targets)..."
    cargo clippy --workspace --lib -- -D warnings 2>&1 || { echo "FAIL: clippy (lib)"; exit 1; }
    ;;

  full)
    echo "Checking workspace build..."
    cargo build --workspace 2>&1 || { echo "FAIL: cargo build --workspace"; exit 1; }
    echo "Checking format..."
    cargo fmt --check --all 2>&1 || { echo "FAIL: cargo fmt --check"; exit 1; }
    echo "Checking clippy (all targets)..."
    cargo clippy --workspace --all-targets -- -D warnings 2>&1 || { echo "FAIL: clippy"; exit 1; }
    echo "Checking rustdoc..."
    RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps 2>&1 || { echo "FAIL: rustdoc"; exit 1; }
    echo "Running workspace tests..."
    cargo test --workspace --all-targets 2>&1 || { echo "FAIL: cargo test"; exit 1; }
    ;;

  scoped)
    crate=""
    tests=()
    while [ $# -gt 0 ]; do
      case "$1" in
        --crate) crate="$2"; shift 2 ;;
        --test) tests+=("$2"); shift 2 ;;
        *) echo "FAIL: unknown scoped argument: $1"; exit 1 ;;
      esac
    done
    [ -n "$crate" ] || { echo "FAIL: scoped mode requires --crate <name>"; exit 1; }
    echo "Checking workspace build (cross-crate compile safety)..."
    cargo build --workspace 2>&1 || { echo "FAIL: cargo build --workspace"; exit 1; }
    echo "Checking format..."
    cargo fmt --check --all 2>&1 || { echo "FAIL: cargo fmt --check"; exit 1; }
    echo "Checking clippy for $crate (lib)..."
    cargo clippy -p "$crate" --lib -- -D warnings 2>&1 || { echo "FAIL: clippy -p $crate"; exit 1; }
    if [ "${#tests[@]}" -eq 0 ]; then
      echo "Running lib tests for $crate..."
      cargo test -p "$crate" --lib 2>&1 || { echo "FAIL: cargo test -p $crate --lib"; exit 1; }
    else
      for t in "${tests[@]}"; do
        echo "Running test binary $crate::$t..."
        cargo test -p "$crate" --test "$t" 2>&1 || { echo "FAIL: cargo test -p $crate --test $t"; exit 1; }
      done
    fi
    ;;

  *)
    echo "FAIL: unknown mode '$mode' (use boot|full|scoped)"; exit 1 ;;
esac

echo "=== smoke PASSED [$mode] ==="
