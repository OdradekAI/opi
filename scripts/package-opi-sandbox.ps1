<#
.SYNOPSIS
Host-neutral opi-sandbox package builder (Phase 16 task 16.15.1).

Builds a distribution archive from an explicit built binary; never invokes opi
and never claims native restriction success (native run is 16.13/16.14.1).

Usage:
    package-opi-sandbox.ps1 -BinaryPath PATH -ArtifactDir PATH          (pack)
    package-opi-sandbox.ps1 -ArtifactDir PATH -Verify                   (verify)

Package layout (under $ArtifactDir):
    package/package.toml        rendered manifest (target + sha256 filled)
    package/bin/opi-sandbox     the executable
    opi-sandbox-<target>.zip    distribution archive (package contents at root)
    extracted/                  clean extraction of the archive
    package-lock.toml           BUILD-TIME audit lock (8 LockMaterial fields)

The archive contains package.toml + bin/ at its root (NO wrapping directory),
matching the package_root that 16.5 install passes to 16.4
validate_executable_contributions. package-lock.toml is an audit artifact;
16.5 recomputes LockMaterial via 16.4 against the extracted package and does
NOT ingest this file. Phase 16 publishes no official Windows package artifact;
this script exists for host-neutral completeness and local verification.

Exit codes: 0 success; 1 layout/hash mismatch or undecodable lock; 2 usage
(missing args, missing/empty binary, rustc unavailable, target undetected).
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string]$ArtifactDir,
    [string]$BinaryPath,
    [switch]$Verify
)

$ErrorActionPreference = 'Stop'
$utf8NoBom = New-Object System.Text.UTF8Encoding($false)

function Fail-Usage([string]$msg) {
    [Console]::Error.WriteLine("package-opi-sandbox: $msg"); exit 2
}
function Fail-Layout([string]$msg) {
    [Console]::Error.WriteLine("package-opi-sandbox: $msg"); exit 1
}
# Lowercase SHA-256 of a file's raw bytes.
function Get-Sha256Path([string]$Path) {
    (Get-FileHash -Algorithm SHA256 -LiteralPath $Path).Hash.ToLowerInvariant()
}
# Lowercase SHA-256 over the LF-normalized bytes of a file (drop every 0x0D),
# matching execution::contribution::lf_normalize + sha256_hex used by 16.4.
function Get-Sha256LfPath([string]$Path) {
    $raw = [System.IO.File]::ReadAllBytes($Path)
    $list = New-Object System.Collections.Generic.List[byte]
    foreach ($b in $raw) { if ($b -ne 13) { $list.Add($b) } }
    $ms = New-Object System.IO.MemoryStream(, $list.ToArray())
    try { (Get-FileHash -Algorithm SHA256 -InputStream $ms).Hash.ToLowerInvariant() }
    finally { $ms.Dispose() }
}
# Read `key = "value"` from the fixed-format build-time lock.
function Read-LockValue([string]$key, [string]$lockText) {
    $m = [regex]::Match($lockText, "$key = `"([^`"]+)`"")
    if ($m.Success) { $m.Groups[1].Value } else { '' }
}

if ($Verify) {
    $Pkg = Join-Path $ArtifactDir 'package'
    $Extracted = Join-Path $ArtifactDir 'extracted'
    $LockPath = Join-Path $ArtifactDir 'package-lock.toml'
    foreach ($rel in @('package.toml', 'bin/opi-sandbox')) {
        if (-not (Test-Path -LiteralPath (Join-Path $Pkg $rel))) {
            Fail-Layout "verify: missing package/$rel"
        }
        if (-not (Test-Path -LiteralPath (Join-Path $Extracted $rel))) {
            Fail-Layout "verify: missing extracted/$rel"
        }
    }
    if (-not (Test-Path -LiteralPath $LockPath)) { Fail-Layout 'verify: missing package-lock.toml' }
    $lockText = [System.IO.File]::ReadAllText($LockPath, $utf8NoBom)
    $declMh = Read-LockValue 'manifest_hash' $lockText
    $declExe = Read-LockValue 'executable_sha256' $lockText
    if (-not $declMh -or -not $declExe) { Fail-Layout 'verify: undecodable lock' }
    $actualMh = Get-Sha256LfPath (Join-Path $Pkg 'package.toml')
    if ($actualMh -cne $declMh) { Fail-Layout 'verify: manifest_hash mismatch' }
    $exePkg = Get-Sha256Path (Join-Path $Pkg 'bin/opi-sandbox')
    $exeExt = Get-Sha256Path (Join-Path $Extracted 'bin/opi-sandbox')
    if ($exePkg -cne $declExe) { Fail-Layout 'verify: package executable sha mismatch' }
    if ($exeExt -cne $declExe) { Fail-Layout 'verify: extracted executable sha mismatch' }
    [Console]::Out.WriteLine("verified opi-sandbox layout: manifest_hash=$actualMh, executable_sha256=$declExe")
    exit 0
}

# --- pack mode ---
if (-not $BinaryPath) { Fail-Usage '-BinaryPath PATH is required in pack mode' }

$ScriptDir = $PSScriptRoot
if (-not $ScriptDir) { $ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path }
$Template = Join-Path $ScriptDir '..\packaging\opi-sandbox\package.toml.template'

if (-not (Test-Path -LiteralPath $BinaryPath -PathType Leaf)) {
    Fail-Usage "binary not found: $BinaryPath"
}
if ((Get-Item -LiteralPath $BinaryPath).Length -eq 0) {
    Fail-Usage "binary is empty: $BinaryPath"
}
if (-not (Test-Path -LiteralPath $Template -PathType Leaf)) {
    Fail-Usage "template not found: $Template"
}

# Detect host target triple from rustc (assumes the supplied -BinaryPath was
# built for this same triple; cross-compiled binaries must be packaged on a
# matching host).
if (-not (Get-Command rustc -ErrorAction SilentlyContinue)) {
    Fail-Usage 'rustc not found on PATH; cannot detect target'
}
$vV = & rustc -vV
if ($LASTEXITCODE -ne 0) { Fail-Usage "rustc -vV failed (exit $LASTEXITCODE)" }
$hostLine = ($vV | Where-Object { $_ -match '^host:' } | Select-Object -First 1)
$Target = ($hostLine -replace '^host:\s*', '').Trim()
if (-not $Target) { Fail-Usage 'could not parse host triple from rustc -vV' }

$ExecSha = Get-Sha256Path $BinaryPath

$null = New-Item -ItemType Directory -Force -Path $ArtifactDir
# Re-packaging wipes prior outputs (clean staging tree, no stale overlay).
$Pkg = Join-Path $ArtifactDir 'package'
$Extracted = Join-Path $ArtifactDir 'extracted'
if (Test-Path -LiteralPath $Pkg) { Remove-Item -Recurse -Force -LiteralPath $Pkg }
if (Test-Path -LiteralPath $Extracted) { Remove-Item -Recurse -Force -LiteralPath $Extracted }
Get-ChildItem -LiteralPath $ArtifactDir -Filter 'opi-sandbox-*.zip' -File -ErrorAction SilentlyContinue |
    Remove-Item -Force -ErrorAction SilentlyContinue
$null = New-Item -ItemType Directory -Force -Path (Join-Path $Pkg 'bin')

# Render the manifest (literal token substitution) and write LF-only UTF-8.
$templateText = [System.IO.File]::ReadAllText($Template, $utf8NoBom)
$rendered = $templateText.Replace('__TARGET__', $Target).Replace('__SHA256__', $ExecSha)
$rendered = $rendered -replace "`r`n", "`n" -replace "`r", "`n"
$PkgToml = Join-Path $Pkg 'package.toml'
[System.IO.File]::WriteAllBytes($PkgToml, $utf8NoBom.GetBytes($rendered))

# Copy the binary into the layout (basename always opi-sandbox; no extension).
Copy-Item -LiteralPath $BinaryPath -Destination (Join-Path $Pkg 'bin/opi-sandbox') -Force

# manifest_hash over the LF-normalized written manifest.
$ManifestHash = Get-Sha256LfPath $PkgToml

# Archive: package contents at root (no wrapping directory).
$Archive = Join-Path $ArtifactDir "opi-sandbox-$Target.zip"
Compress-Archive -Path (Join-Path $Pkg '*') -DestinationPath $Archive -Force

# Clean extracted staging tree.
$null = New-Item -ItemType Directory -Force -Path $Extracted
Expand-Archive -Path $Archive -DestinationPath $Extracted -Force

# Self-verify (defense-in-depth against a copy/archive bug).
$extractedSha = Get-Sha256Path (Join-Path $Extracted 'bin/opi-sandbox')
if ($extractedSha -cne $ExecSha) { Fail-Layout 'extracted binary hash mismatch' }

# Build-time audit lock (flat LockMaterial table), LF-only.
$lock = @"
# Build-time audit lock for the opi-sandbox package. NOT consumed by 16.5;
# 16.5 recomputes LockMaterial via 16.4 against the extracted package.
manifest_hash = "$ManifestHash"
executable_rel_path = "bin/opi-sandbox"
executable_sha256 = "$ExecSha"
package_version = "0.8.0"
target = "$Target"
opi_range = ">=0.8,<0.9"
protocol = "command-execution-jsonl-v1"
adapter_id = "opi-sandbox"
"@
$lock = $lock -replace "`r`n", "`n"
[System.IO.File]::WriteAllBytes((Join-Path $ArtifactDir 'package-lock.toml'), $utf8NoBom.GetBytes($lock))

[Console]::Out.WriteLine("packaged opi-sandbox for ${Target}: sha256=$ExecSha, layout=$Pkg")
exit 0
