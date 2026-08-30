#!/usr/bin/env bash
set -euo pipefail

# Parameterized opi-implement mechanical gates.
#
# boot:   A.3 format + production lib/bin clippy; no tests or standalone build.
# full:   workspace-tier D.1 and the authoritative Phase-exit order:
#         documentation contract, format, all-target clippy, workspace tests,
#         doc tests, then rustdoc - each exactly once, in that order.
# scoped: non-workspace D.1; one crate's production targets, rustdoc, and named
#         test binaries (or lib tests when none are named).
#
# CARGO_TARGET_DIR is honored. The skill assigns a persistent external cache
# keyed by worktree and toolchain; this script never cleans it.

mode="${1:-boot}"
shift || true

echo "=== opi-impl smoke [$mode] ==="

rustc --version >/dev/null 2>&1 || { echo "FAIL: rustc not found"; exit 1; }
cargo --version >/dev/null 2>&1 || { echo "FAIL: cargo not found"; exit 1; }

cache_leased=0
cache_tool="$(cd "$(dirname "$0")" && pwd)/opi-cargo-cache.py"
if [ -z "${CARGO_TARGET_DIR:-}" ]; then
  export CARGO_TARGET_DIR="$(python "$cache_tool" resolve)"
  python "$cache_tool" lease start --target "$CARGO_TARGET_DIR" --pid "$$"
  cache_leased=1
fi
release_cache_lease() {
  if [ "$cache_leased" -eq 1 ]; then
    python "$cache_tool" lease end --target "$CARGO_TARGET_DIR" --pid "$$" >/dev/null 2>&1 || true
  fi
}
trap release_cache_lease EXIT

case "$mode" in
  boot)
    echo "Checking format..."
    cargo fmt --check --all 2>&1 || { echo "FAIL: cargo fmt --check"; exit 1; }
    echo "Checking clippy (production lib/bin targets)..."
    cargo clippy --workspace --lib --bins -- -D warnings 2>&1 || {
      echo "FAIL: clippy (production targets)"; exit 1;
    }
    ;;

  full)
    echo "Checking documentation contract..."
    python scripts/opi-doc-check.py 2>&1 || { echo "FAIL: opi-doc-check"; exit 1; }
    echo "Checking format..."
    cargo fmt --check --all 2>&1 || { echo "FAIL: cargo fmt --check"; exit 1; }
    echo "Checking clippy (all targets)..."
    cargo clippy --workspace --all-targets -- -D warnings 2>&1 || { echo "FAIL: clippy"; exit 1; }
    echo "Running workspace tests..."
    cargo test --workspace --all-targets 2>&1 || { echo "FAIL: cargo test"; exit 1; }
    echo "Running doc tests..."
    cargo test --workspace --doc 2>&1 || { echo "FAIL: cargo test --doc"; exit 1; }
    echo "Checking rustdoc..."
    RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps 2>&1 || { echo "FAIL: rustdoc"; exit 1; }
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

    echo "Checking format..."
    cargo fmt --check --all 2>&1 || { echo "FAIL: cargo fmt --check"; exit 1; }
    echo "Checking clippy for $crate (production lib/bin targets)..."
    cargo clippy -p "$crate" --lib --bins -- -D warnings 2>&1 || {
      echo "FAIL: clippy -p $crate"; exit 1;
    }
    echo "Checking rustdoc for $crate..."
    RUSTDOCFLAGS="-D warnings" cargo doc -p "$crate" --no-deps 2>&1 || {
      echo "FAIL: rustdoc -p $crate"; exit 1;
    }
    if [ "${#tests[@]}" -eq 0 ]; then
      echo "Running lib tests for $crate..."
      cargo test -p "$crate" --lib 2>&1 || { echo "FAIL: cargo test -p $crate --lib"; exit 1; }
    else
      for test_name in "${tests[@]}"; do
        echo "Checking clippy for test binary $crate::$test_name..."
        cargo clippy -p "$crate" --test "$test_name" -- -D warnings 2>&1 || {
          echo "FAIL: clippy -p $crate --test $test_name"; exit 1;
        }
        echo "Running test binary $crate::$test_name..."
        cargo test -p "$crate" --test "$test_name" 2>&1 || {
          echo "FAIL: cargo test -p $crate --test $test_name"; exit 1;
        }
      done
    fi
    ;;

  *)
    echo "FAIL: unknown mode '$mode' (use boot|full|scoped)"; exit 1 ;;
esac

echo "=== smoke PASSED [$mode] ==="
