#Requires -Version 5.1
Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

# opi-implement smoke — parameterized verify gates.
# See scripts/opi-impl-smoke.sh for the mode reference (boot | full | scoped).
# CARGO_TARGET_DIR is honored from the environment (per-session dir off the repo drive).

$mode = if ($args.Count -gt 0) { $args[0] } else { "boot" }
$rest = if ($args.Count -gt 1) { $args[1..($args.Count - 1)] } else { @() }

Write-Host "=== opi-impl smoke [$mode] ==="

try { rustc --version | Out-Null } catch { Write-Error "FAIL: rustc not found"; exit 1 }
try { cargo --version | Out-Null } catch { Write-Error "FAIL: cargo not found"; exit 1 }

switch ($mode) {
  "boot" {
    Write-Host "Checking workspace build..."
    cargo build --workspace
    if ($LASTEXITCODE -ne 0) { Write-Error "FAIL: cargo build --workspace"; exit 1 }
    Write-Host "Checking format..."
    cargo fmt --check --all
    if ($LASTEXITCODE -ne 0) { Write-Error "FAIL: cargo fmt --check"; exit 1 }
    Write-Host "Checking clippy (lib targets)..."
    cargo clippy --workspace --lib -- -D warnings
    if ($LASTEXITCODE -ne 0) { Write-Error "FAIL: clippy (lib)"; exit 1 }
  }

  "full" {
    Write-Host "Checking workspace build..."
    cargo build --workspace
    if ($LASTEXITCODE -ne 0) { Write-Error "FAIL: cargo build --workspace"; exit 1 }
    Write-Host "Checking format..."
    cargo fmt --check --all
    if ($LASTEXITCODE -ne 0) { Write-Error "FAIL: cargo fmt --check"; exit 1 }
    Write-Host "Checking clippy (all targets)..."
    cargo clippy --workspace --all-targets -- -D warnings
    if ($LASTEXITCODE -ne 0) { Write-Error "FAIL: clippy"; exit 1 }
    Write-Host "Checking rustdoc..."
    $env:RUSTDOCFLAGS = "-D warnings"; cargo doc --workspace --no-deps; Remove-Item Env:RUSTDOCFLAGS
    if ($LASTEXITCODE -ne 0) { Write-Error "FAIL: rustdoc"; exit 1 }
    Write-Host "Running workspace tests..."
    cargo test --workspace --all-targets
    if ($LASTEXITCODE -ne 0) { Write-Error "FAIL: cargo test"; exit 1 }
  }

  "scoped" {
    $crate = $null
    $tests = @()
    for ($i = 0; $i -lt $rest.Count; $i++) {
      switch ($rest[$i]) {
        "--crate" { $crate = $rest[$i + 1]; $i++ }
        "--test" { $tests += $rest[$i + 1]; $i++ }
        default { Write-Error "FAIL: unknown scoped argument: $($rest[$i])"; exit 1 }
      }
    }
    if (-not $crate) { Write-Error "FAIL: scoped mode requires --crate <name>"; exit 1 }
    Write-Host "Checking workspace build (cross-crate compile safety)..."
    cargo build --workspace
    if ($LASTEXITCODE -ne 0) { Write-Error "FAIL: cargo build --workspace"; exit 1 }
    Write-Host "Checking format..."
    cargo fmt --check --all
    if ($LASTEXITCODE -ne 0) { Write-Error "FAIL: cargo fmt --check"; exit 1 }
    Write-Host "Checking clippy for $crate (lib)..."
    cargo clippy -p $crate --lib -- -D warnings
    if ($LASTEXITCODE -ne 0) { Write-Error "FAIL: clippy -p $crate"; exit 1 }
    if ($tests.Count -eq 0) {
      Write-Host "Running lib tests for $crate..."
      cargo test -p $crate --lib
      if ($LASTEXITCODE -ne 0) { Write-Error "FAIL: cargo test -p $crate --lib"; exit 1 }
    } else {
      foreach ($t in $tests) {
        Write-Host "Running test binary ${crate}::$t..."
        cargo test -p $crate --test $t
        if ($LASTEXITCODE -ne 0) { Write-Error "FAIL: cargo test -p $crate --test $t"; exit 1 }
      }
    }
  }

  default { Write-Error "FAIL: unknown mode '$mode' (use boot|full|scoped)"; exit 1 }
}

Write-Host "=== smoke PASSED [$mode] ==="
