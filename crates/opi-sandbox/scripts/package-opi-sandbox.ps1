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
    package/schemas/command-execution-jsonl-v1.schema.json
    package/licenses/LICENSE    project license
    opi-sandbox-<target>.zip    distribution archive (package contents at root)
    extracted/                  clean extraction of the archive
    package-lock.toml           BUILD-TIME audit lock (8 LockMaterial fields)
    target                      exact target triple for artifact audit

The archive contains package.toml + bin/ + schemas/ + licenses/ at its root
(NO wrapping directory),
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
Add-Type -AssemblyName System.IO.Compression
Add-Type -AssemblyName System.IO.Compression.FileSystem

function Fail-Usage([string]$msg) {
    [Console]::Error.WriteLine("package-opi-sandbox: $msg"); exit 2
}
function Fail-Layout([string]$msg) {
    [Console]::Error.WriteLine("package-opi-sandbox: $msg"); exit 1
}
# Lowercase SHA-256 of a file's raw bytes. Computes via .NET directly because
# the Get-FileHash cmdlet is not resolvable in the non-interactive powershell.exe
# session that the opi_sandbox_packaging test spawns on windows-latest CI
# runners; the output is byte-identical to (Get-FileHash -Algorithm SHA256).
function Get-Sha256Path([string]$Path) {
    $sha = [System.Security.Cryptography.SHA256]::Create()
    try {
        $hash = $sha.ComputeHash([System.IO.File]::ReadAllBytes($Path))
    } finally {
        $sha.Dispose()
    }
    (-join ($hash | ForEach-Object { $_.ToString('x2') }))
}
$ScriptDir = $PSScriptRoot
if (-not $ScriptDir) { $ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path }
$PackageHelper = Join-Path $ScriptDir 'opi-sandbox-package.py'
$Template = Join-Path $ScriptDir '..\packaging\package.toml.template'
$WorkspaceManifest = Join-Path $ScriptDir '..\..\..\Cargo.toml'
$SchemaSnapshot = Join-Path $ScriptDir '..\..\opi-protocol\tests\snapshots\execution_v1_schema__schema_v1.snap'
$LicenseFile = Join-Path $ScriptDir '..\..\..\LICENSE'

if ($Verify) {
    & python $PackageHelper verify --artifact-dir $ArtifactDir --archive-suffix '.zip' --workspace-license $LicenseFile --schema-snapshot $SchemaSnapshot
    exit $LASTEXITCODE
}

# --- pack mode ---
if (-not $BinaryPath) { Fail-Usage '-BinaryPath PATH is required in pack mode' }

if (-not (Test-Path -LiteralPath $BinaryPath -PathType Leaf)) {
    Fail-Usage "binary not found: $BinaryPath"
}
if ((Get-Item -LiteralPath $BinaryPath).Length -eq 0) {
    Fail-Usage "binary is empty: $BinaryPath"
}
if (-not (Test-Path -LiteralPath $Template -PathType Leaf) -or -not (Test-Path -LiteralPath $PackageHelper -PathType Leaf)) {
    Fail-Usage "template not found: $Template"
}
if (-not (Test-Path -LiteralPath $WorkspaceManifest -PathType Leaf)) {
    Fail-Usage "workspace manifest not found: $WorkspaceManifest"
}
if (-not (Test-Path -LiteralPath $SchemaSnapshot -PathType Leaf) -or -not (Test-Path -LiteralPath $LicenseFile -PathType Leaf)) {
    Fail-Usage 'schema snapshot or LICENSE is missing'
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

& python $PackageHelper validate-executable --binary $BinaryPath --target $Target
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
$ExecSha = Get-Sha256Path $BinaryPath

$null = New-Item -ItemType Directory -Force -Path $ArtifactDir
# Re-packaging wipes prior outputs (clean staging tree, no stale overlay).
$Pkg = Join-Path $ArtifactDir 'package'
$Extracted = Join-Path $ArtifactDir 'extracted'
if (Test-Path -LiteralPath $Pkg) { Remove-Item -Recurse -Force -LiteralPath $Pkg }
if (Test-Path -LiteralPath $Extracted) { Remove-Item -Recurse -Force -LiteralPath $Extracted }
Get-ChildItem -LiteralPath $ArtifactDir -Filter 'opi-sandbox-*.zip' -File -ErrorAction SilentlyContinue |
    Remove-Item -Force -ErrorAction SilentlyContinue
Remove-Item -Force -LiteralPath (Join-Path $ArtifactDir 'target') -ErrorAction SilentlyContinue
$null = New-Item -ItemType Directory -Force -Path (Join-Path $Pkg 'bin')
$null = New-Item -ItemType Directory -Force -Path (Join-Path $Pkg 'schemas')
$null = New-Item -ItemType Directory -Force -Path (Join-Path $Pkg 'licenses')

$PkgToml = Join-Path $Pkg 'package.toml'
$PackageMeta = Join-Path $ArtifactDir 'package-meta.txt'
& python $PackageHelper render --workspace-manifest $WorkspaceManifest --template $Template --target $Target --sha256 $ExecSha --output $PkgToml --metadata-output $PackageMeta
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
$metadata = [System.IO.File]::ReadAllLines($PackageMeta, $utf8NoBom)
if ($metadata.Count -ne 2) { Fail-Usage 'invalid package metadata output' }
$PackageVersion = $metadata[0]
$OpiRange = $metadata[1]
Remove-Item -Force -LiteralPath $PackageMeta

# Copy the binary into the layout (basename always opi-sandbox; no extension).
Copy-Item -LiteralPath $BinaryPath -Destination (Join-Path $Pkg 'bin/opi-sandbox') -Force

# Strip insta's metadata header from the reviewed opi-protocol schema snapshot.
$snapshotLines = [System.IO.File]::ReadAllLines($SchemaSnapshot, $utf8NoBom)
$markers = @()
for ($i = 0; $i -lt $snapshotLines.Length; $i++) {
    if ($snapshotLines[$i] -ceq '---') { $markers += $i }
}
if ($markers.Count -lt 2 -or $markers[1] + 1 -ge $snapshotLines.Length) {
    Fail-Usage 'invalid protocol schema snapshot header'
}
$schemaText = (($snapshotLines[($markers[1] + 1)..($snapshotLines.Length - 1)]) -join "`n") + "`n"
try { $null = $schemaText | ConvertFrom-Json } catch { Fail-Usage 'invalid protocol schema JSON' }
[System.IO.File]::WriteAllBytes(
    (Join-Path $Pkg 'schemas/command-execution-jsonl-v1.schema.json'),
    $utf8NoBom.GetBytes($schemaText)
)
Copy-Item -LiteralPath $LicenseFile -Destination (Join-Path $Pkg 'licenses/LICENSE') -Force

# manifest_hash over the exact written manifest bytes.
$ManifestHash = Get-Sha256Path $PkgToml

# Archive: package contents at root (no wrapping directory).
$Archive = Join-Path $ArtifactDir "opi-sandbox-$Target.zip"
$zip = [System.IO.Compression.ZipFile]::Open($Archive, [System.IO.Compression.ZipArchiveMode]::Create)
try {
    foreach ($rel in @(
        'package.toml',
        'bin/opi-sandbox',
        'schemas/command-execution-jsonl-v1.schema.json',
        'licenses/LICENSE'
    )) {
        $entryName = $rel.Replace('\', '/')
        [System.IO.Compression.ZipFileExtensions]::CreateEntryFromFile(
            $zip,
            (Join-Path $Pkg $rel),
            $entryName,
            [System.IO.Compression.CompressionLevel]::Optimal
        ) | Out-Null
    }
} finally {
    $zip.Dispose()
}

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
package_version = "$PackageVersion"
target = "$Target"
opi_range = "$OpiRange"
protocol = "command-execution-jsonl-v1"
adapter_id = "opi-sandbox"
"@
$lock = $lock -replace "`r`n", "`n"
[System.IO.File]::WriteAllBytes((Join-Path $ArtifactDir 'package-lock.toml'), $utf8NoBom.GetBytes($lock))
[System.IO.File]::WriteAllBytes((Join-Path $ArtifactDir 'target'), $utf8NoBom.GetBytes("$Target`n"))

& python $PackageHelper verify --artifact-dir $ArtifactDir --archive-suffix '.zip' --workspace-license $LicenseFile --schema-snapshot $SchemaSnapshot
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

[Console]::Out.WriteLine("packaged opi-sandbox for ${Target}: sha256=$ExecSha, layout=$Pkg")
exit 0
