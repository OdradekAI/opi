#Requires -Version 5.1
Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

# Parameterized opi-implement mechanical gates. CARGO_TARGET_DIR is honored;
# the skill assigns a persistent external per-worktree/toolchain cache.

$mode = if ($args.Count -gt 0) { $args[0] } else { "boot" }
$rest = if ($args.Count -gt 1) { $args[1..($args.Count - 1)] } else { @() }

Write-Host "=== opi-impl smoke [$mode] ==="

try { rustc --version | Out-Null } catch { Write-Error "FAIL: rustc not found"; exit 1 }
try { cargo --version | Out-Null } catch { Write-Error "FAIL: cargo not found"; exit 1 }

$cacheLeased = $false
$cacheTool = Join-Path $PSScriptRoot "opi-cargo-cache.py"
if (-not $env:CARGO_TARGET_DIR) {
  $env:CARGO_TARGET_DIR = (& python $cacheTool resolve).Trim()
  if ($LASTEXITCODE -ne 0) { Write-Error "FAIL: resolve Cargo cache"; exit 1 }
  python $cacheTool lease start --target $env:CARGO_TARGET_DIR --pid $PID
  if ($LASTEXITCODE -ne 0) { Write-Error "FAIL: lease Cargo cache"; exit 1 }
  $cacheLeased = $true
}

try {
switch ($mode) {
  "boot" {
    Write-Host "Checking format..."
    cargo fmt --check --all
    if ($LASTEXITCODE -ne 0) { Write-Error "FAIL: cargo fmt --check"; exit 1 }
    Write-Host "Checking clippy (production lib/bin targets)..."
    cargo clippy --workspace --lib --bins -- -D warnings
    if ($LASTEXITCODE -ne 0) { Write-Error "FAIL: clippy (production targets)"; exit 1 }
  }

  "full" {
    Write-Host "Checking documentation contract..."
    python scripts/opi-doc-check.py
    if ($LASTEXITCODE -ne 0) { Write-Error "FAIL: opi-doc-check"; exit 1 }
    Write-Host "Checking format..."
    cargo fmt --check --all
    if ($LASTEXITCODE -ne 0) { Write-Error "FAIL: cargo fmt --check"; exit 1 }
    Write-Host "Checking clippy (all targets)..."
    cargo clippy --workspace --all-targets -- -D warnings
    if ($LASTEXITCODE -ne 0) { Write-Error "FAIL: clippy"; exit 1 }
    Write-Host "Running workspace tests..."
    cargo test --workspace --all-targets
    if ($LASTEXITCODE -ne 0) { Write-Error "FAIL: cargo test"; exit 1 }
    Write-Host "Running doc tests..."
    cargo test --workspace --doc
    if ($LASTEXITCODE -ne 0) { Write-Error "FAIL: cargo test --doc"; exit 1 }
    Write-Host "Checking rustdoc..."
    $env:RUSTDOCFLAGS = "-D warnings"
    cargo doc --workspace --no-deps
    Remove-Item Env:RUSTDOCFLAGS
    if ($LASTEXITCODE -ne 0) { Write-Error "FAIL: rustdoc"; exit 1 }
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

    Write-Host "Checking format..."
    cargo fmt --check --all
    if ($LASTEXITCODE -ne 0) { Write-Error "FAIL: cargo fmt --check"; exit 1 }
    Write-Host "Checking clippy for $crate (production lib/bin targets)..."
    cargo clippy -p $crate --lib --bins -- -D warnings
    if ($LASTEXITCODE -ne 0) { Write-Error "FAIL: clippy -p $crate"; exit 1 }
    Write-Host "Checking rustdoc for $crate..."
    $env:RUSTDOCFLAGS = "-D warnings"
    cargo doc -p $crate --no-deps
    Remove-Item Env:RUSTDOCFLAGS
    if ($LASTEXITCODE -ne 0) { Write-Error "FAIL: rustdoc -p $crate"; exit 1 }

    if ($tests.Count -eq 0) {
      Write-Host "Running lib tests for $crate..."
      cargo test -p $crate --lib
      if ($LASTEXITCODE -ne 0) { Write-Error "FAIL: cargo test -p $crate --lib"; exit 1 }
    } else {
      foreach ($testName in $tests) {
        Write-Host "Checking clippy for test binary ${crate}::$testName..."
        cargo clippy -p $crate --test $testName -- -D warnings
        if ($LASTEXITCODE -ne 0) { Write-Error "FAIL: clippy -p $crate --test $testName"; exit 1 }
        Write-Host "Running test binary ${crate}::$testName..."
        cargo test -p $crate --test $testName
        if ($LASTEXITCODE -ne 0) { Write-Error "FAIL: cargo test -p $crate --test $testName"; exit 1 }
      }
    }
  }

  default { Write-Error "FAIL: unknown mode '$mode' (use boot|full|scoped)"; exit 1 }
}
} finally {
  if ($cacheLeased) {
    python $cacheTool lease end --target $env:CARGO_TARGET_DIR --pid $PID | Out-Null
  }
}

Write-Host "=== smoke PASSED [$mode] ==="
