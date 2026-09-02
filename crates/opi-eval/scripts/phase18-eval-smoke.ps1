# Phase 18 assembled-run smoke wrapper (task 18.12), Windows PowerShell.
#
# Runs the production `opi-eval run` path twice (a completed run and a
# native-verifier failure), then the offline `regrade`/`report` commands
# (task 18.13) over the completed run root, and preserves the exact
# command, stdout, stderr, exit code, receipt bytes, and bundle identities
# under an output root. Exits 0 only when every observed outcome matches
# the pinned expectations.
#
# Usage:
#   powershell -File crates\opi-eval\scripts\phase18-eval-smoke.ps1 [-Fixtures DIR] [-Out DIR]
#   powershell -File crates\opi-eval\scripts\phase18-eval-smoke.ps1 -Mode report -Bundle RUN_ROOT -ArtifactDir DIR

param(
    [string]$Mode = "all",
    [string]$Bundle = "",
    [string]$ArtifactDir = "",
    [string]$Fixtures = (Join-Path $PSScriptRoot "..\tests\fixtures"),
    [string]$Out = (Join-Path ([System.IO.Path]::GetTempPath()) ("phase18-eval-smoke-" + [System.IO.Path]::GetRandomFileName()))
)

$ErrorActionPreference = 'Stop'
$Config = Join-Path $Fixtures "experiment\phase18-local.toml"

function Get-Sha256 {
    param([string]$Path)
    $stream = [System.IO.File]::OpenRead($Path)
    $sha256 = [System.Security.Cryptography.SHA256]::Create()
    try {
        return ([System.BitConverter]::ToString($sha256.ComputeHash($stream))).Replace("-", "").ToLowerInvariant()
    }
    finally {
        $sha256.Dispose()
        $stream.Dispose()
    }
}

function Invoke-Case {
    param([string]$Behavior, [int]$Expected)
    $caseDir = Join-Path $Out $Behavior
    New-Item -ItemType Directory -Force -Path $caseDir | Out-Null
    # The exact command, preserved for audit.
    $argv = @("run", "-q", "-p", "opi-eval", "--", "run", "--config", $Config,
        "--root", (Join-Path $caseDir "root"), "--fixtures", $Fixtures, "--behavior", $Behavior)
    "cargo $($argv -join ' ')" | Set-Content -Encoding ascii (Join-Path $caseDir "command.txt")
    $process = Start-Process -FilePath cargo -ArgumentList $argv -NoNewWindow -Wait -PassThru `
        -RedirectStandardOutput (Join-Path $caseDir "stdout.json") `
        -RedirectStandardError (Join-Path $caseDir "stderr.txt")
    "$($process.ExitCode)" | Set-Content -Encoding ascii (Join-Path $caseDir "exit_code")
    if ($process.ExitCode -ne $Expected) {
        Write-Error "behavior ${Behavior}: expected exit ${Expected}, observed $($process.ExitCode)"
    }
    return $process.ExitCode
}

function Invoke-Offline {
    param([string]$BundleRoot, [string]$Target)
    New-Item -ItemType Directory -Force -Path $Target | Out-Null
    # Offline regrade: exact command, stdout, stderr, exit code preserved.
    $argv = @("run", "-q", "-p", "opi-eval", "--", "regrade", "--root", $BundleRoot)
    "cargo $($argv -join ' ')" | Set-Content -Encoding ascii (Join-Path $Target "regrade-command.txt")
    $process = Start-Process -FilePath cargo -ArgumentList $argv -NoNewWindow -Wait -PassThru `
        -RedirectStandardOutput (Join-Path $Target "regrade-stdout.json") `
        -RedirectStandardError (Join-Path $Target "regrade-stderr.txt")
    "$($process.ExitCode)" | Set-Content -Encoding ascii (Join-Path $Target "regrade-exit_code")
    if ($process.ExitCode -ne 0) {
        Write-Error "regrade: expected exit 0, observed $($process.ExitCode)"
        return 1
    }
    # Offline report: rendered twice; the second render must be byte-stable.
    $argv = @("run", "-q", "-p", "opi-eval", "--", "report", "--root", $BundleRoot,
        "--out", (Join-Path $Target "report-1.json"))
    "cargo $($argv -join ' ')" | Set-Content -Encoding ascii (Join-Path $Target "report-command.txt")
    $process = Start-Process -FilePath cargo -ArgumentList $argv -NoNewWindow -Wait -PassThru `
        -RedirectStandardOutput (Join-Path $Target "report-stdout.json") `
        -RedirectStandardError (Join-Path $Target "report-stderr.txt")
    "$($process.ExitCode)" | Set-Content -Encoding ascii (Join-Path $Target "report-exit_code")
    if ($process.ExitCode -ne 0) {
        Write-Error "report: expected exit 0, observed $($process.ExitCode)"
        return 1
    }
    $argv = @("run", "-q", "-p", "opi-eval", "--", "report", "--root", $BundleRoot,
        "--out", (Join-Path $Target "report-2.json"))
    $process = Start-Process -FilePath cargo -ArgumentList $argv -NoNewWindow -Wait -PassThru
    if ($process.ExitCode -ne 0) {
        Write-Error "report render 2: expected exit 0, observed $($process.ExitCode)"
        return 1
    }
    $first = Get-Content (Join-Path $Target "report-1.json") -Raw -Encoding UTF8
    $second = Get-Content (Join-Path $Target "report-2.json") -Raw -Encoding UTF8
    if ($first -cne $second) {
        Write-Error "report renders are not byte-stable"
        return 1
    }
    return 0
}

if ($Mode -eq "report") {
    # Offline-only mode over a caller-provided sealed run root.
    if (-not $Bundle -or -not $ArtifactDir) {
        Write-Error "report mode requires -Bundle and -ArtifactDir"
        exit 2
    }
    $offlineCode = Invoke-Offline -BundleRoot $Bundle -Target $ArtifactDir
    if ($offlineCode -ne 0) { exit 1 }
    Write-Output $ArtifactDir
    exit 0
}

New-Item -ItemType Directory -Force -Path $Out | Out-Null
Invoke-Case -Behavior happy -Expected 0 | Out-Null
Invoke-Case -Behavior verifier-failure -Expected 1 | Out-Null
$offlineCode = Invoke-Offline -BundleRoot (Join-Path $Out "happy\root") -Target (Join-Path $Out "offline")
if ($offlineCode -ne 0) { exit 1 }

# Artifact audit: preserved receipts, sealed bundles, and content-addressed
# bundle identities for the completed run.
$rows = @()
Get-ChildItem (Join-Path $Out "happy\root\trials") -Directory | Sort-Object Name | ForEach-Object {
    $receipt = Join-Path $_.FullName "receipt.json"
    $receiptJson = Get-Content $receipt -Raw -Encoding UTF8 | ConvertFrom-Json
    $sealed = Test-Path (Join-Path $_.FullName "bundle\manifest.json")
    $rows += [ordered]@{
        trial            = $_.Name
        bundle_identity  = $receiptJson.bundle_identity
        receipt_sha256   = Get-Sha256 -Path $receipt
        sealed           = $sealed
    }
}
$receiptWritten = Test-Path (Join-Path $Out "verifier-failure\root\trials\trial-opi-1\receipt.json")
$regradeExit = (Get-Content (Join-Path $Out "offline\regrade-exit_code") -Raw -Encoding UTF8).Trim()
$reportSha = Get-Sha256 -Path (Join-Path $Out "offline\report-1.json")
$byteStable = ((Get-Content (Join-Path $Out "offline\report-1.json") -Raw -Encoding UTF8) -ceq `
    (Get-Content (Join-Path $Out "offline\report-2.json") -Raw -Encoding UTF8))
$audit = [ordered]@{
    schema          = "phase18-eval-smoke-audit/1"
    happy           = [ordered]@{ bundle_identities = $rows }
    verifier_failure = [ordered]@{ receipt_written = $receiptWritten }
    offline         = [ordered]@{
        regrade_exit  = [int]$regradeExit
        report_sha256 = $reportSha
        byte_stable   = $byteStable
    }
}
$auditJson = $audit | ConvertTo-Json -Depth 5
$utf8NoBom = [System.Text.UTF8Encoding]::new($false)
[System.IO.File]::WriteAllText((Join-Path $Out "audit.json"), $auditJson, $utf8NoBom)
Write-Output $Out
exit 0
