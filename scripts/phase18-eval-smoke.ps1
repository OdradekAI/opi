# Phase 18 assembled-run smoke wrapper (task 18.12), Windows PowerShell.
#
# Runs the production `opi-eval run` path twice (a completed run and a
# native-verifier failure) and preserves the exact command, stdout, stderr,
# exit code, receipt bytes, and bundle identities under an output root.
# Exits 0 only when both observed outcomes match the pinned expectations.
#
# Usage: powershell -File scripts\phase18-eval-smoke.ps1 [-Fixtures DIR] [-Out DIR]

param(
    [string]$Fixtures = (Join-Path $PSScriptRoot "..\crates\opi-eval\tests\fixtures"),
    [string]$Out = (Join-Path ([System.IO.Path]::GetTempPath()) ("phase18-eval-smoke-" + [System.IO.Path]::GetRandomFileName()))
)

$ErrorActionPreference = 'Stop'
$Config = Join-Path $Fixtures "experiment\phase18-local.toml"

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

New-Item -ItemType Directory -Force -Path $Out | Out-Null
Invoke-Case -Behavior happy -Expected 0 | Out-Null
Invoke-Case -Behavior verifier-failure -Expected 1 | Out-Null

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
        receipt_sha256   = (Get-FileHash $receipt -Algorithm SHA256).Hash.ToLowerInvariant()
        sealed           = $sealed
    }
}
$receiptWritten = Test-Path (Join-Path $Out "verifier-failure\root\trials\trial-opi-1\receipt.json")
$audit = [ordered]@{
    schema          = "phase18-eval-smoke-audit/1"
    happy           = [ordered]@{ bundle_identities = $rows }
    verifier_failure = [ordered]@{ receipt_written = $receiptWritten }
}
$audit | ConvertTo-Json -Depth 5 | Set-Content -Encoding utf8 (Join-Path $Out "audit.json")
Write-Output $Out
exit 0
