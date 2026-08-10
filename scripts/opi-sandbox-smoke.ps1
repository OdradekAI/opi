# Standalone acceptance smoke for the opi-sandbox binary (Phase 16 task 16.11.2).
# Launches ONLY the explicit -BinaryPath; never invokes cargo or opi.
#
# Usage: opi-sandbox-smoke.ps1 -BinaryPath PATH -ArtifactDir PATH
#
# Windows retains the Phase 16 unsupported/no-artifact posture. Native direct
# and backend archive evidence is produced only by the Linux/macOS script.
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string]$BinaryPath,
    [Parameter(Mandatory = $true)][string]$ArtifactDir
)

$ErrorActionPreference = 'Stop'

if (-not (Test-Path $BinaryPath)) {
    throw "opi-sandbox-smoke: binary not found: $BinaryPath"
}
New-Item -ItemType Directory -Force -Path $ArtifactDir | Out-Null
$BinaryPath = (Resolve-Path -LiteralPath $BinaryPath).Path
$ArtifactDir = (Resolve-Path -LiteralPath $ArtifactDir).Path

# Run an isolated copy from a distinct empty working directory. Neither the
# supplied binary's build directory nor the caller's current directory is part
# of the standalone acceptance surface.
$IsolationRoot = Join-Path $ArtifactDir ("isolated-" + [guid]::NewGuid().ToString('N'))
$IsolatedBinDir = Join-Path $IsolationRoot 'bin'
$IsolatedCwd = Join-Path $IsolationRoot 'cwd'
New-Item -ItemType Directory -Path $IsolatedBinDir | Out-Null
New-Item -ItemType Directory -Path $IsolatedCwd | Out-Null
$IsolatedBinary = Join-Path $IsolatedBinDir 'opi-sandbox.exe'
Copy-Item -LiteralPath $BinaryPath -Destination $IsolatedBinary

# Isolation: scrub opi from PATH; point Opi config/session/package/model env at
# sentinel locations under the artifact dir. The binary must ignore all of them.
$Sentinel = Join-Path $ArtifactDir 'sentinel'
New-Item -ItemType Directory -Force -Path (Join-Path $Sentinel 'opi') | Out-Null
$CanaryPath = Join-Path $Sentinel 'opi\config.toml'
Set-Content -Path $CanaryPath -Value 'CANARY-opi-config-not-read' -Encoding ascii
$env:HOME = $Sentinel
$env:XDG_CONFIG_HOME = $Sentinel
$env:APPDATA = $Sentinel
$env:OPI_CONFIG_DIR = $Sentinel
$env:OPI_SESSIONS_DIR = Join-Path $Sentinel 'sessions'
$env:OPI_PACKAGE_STORE = Join-Path $Sentinel 'store'
$env:OPI_MODEL = 'sentinel-model-not-used'
# Rebuild PATH excluding any directory containing opi.exe or opi.
$env:PATH = (($env:PATH -split ';') |
        Where-Object { $_ -and -not (Test-Path (Join-Path $_ 'opi.exe')) -and -not (Test-Path (Join-Path $_ 'opi')) }) -join ';'

# 1. --help
Push-Location $IsolatedCwd
try {
& $IsolatedBinary --help | Out-File -FilePath (Join-Path $ArtifactDir 'help.txt') -Encoding ascii
$help = Get-Content (Join-Path $ArtifactDir 'help.txt') -Raw
if (-not ($help -match 'run') -or -not ($help -match 'doctor')) {
    throw "opi-sandbox-smoke: --help missing run/doctor"
}

# 2. --version
& $IsolatedBinary --version | Out-File -FilePath (Join-Path $ArtifactDir 'version.txt') -Encoding ascii
if (-not ((Get-Content (Join-Path $ArtifactDir 'version.txt') -Raw) -match 'opi-sandbox')) {
    throw "opi-sandbox-smoke: --version missing opi-sandbox"
}

# 3. doctor --json (stable object; supported=false everywhere in 16.11.2)
& $IsolatedBinary doctor --json | Out-File -FilePath (Join-Path $ArtifactDir 'doctor.json') -Encoding ascii
$doc = Get-Content (Join-Path $ArtifactDir 'doctor.json') -Raw | ConvertFrom-Json
if ($doc.schema_version -ne 1) { throw "opi-sandbox-smoke: doctor schema_version=$($doc.schema_version)" }
if ($doc.supported -ne $false) { throw "opi-sandbox-smoke: doctor supported=$($doc.supported) (must be false in 16.11.2)" }
if ($doc.target -ne 'windows') { throw "opi-sandbox-smoke: doctor target=$($doc.target) (must be windows)" }
if (@($doc.mechanisms).Count -ne 0) { throw "opi-sandbox-smoke: doctor mechanisms not empty" }
if (-not (@($doc.profiles) -contains 'workspace-write')) { throw "opi-sandbox-smoke: doctor profiles missing workspace-write" }

# 4. run with a VALID argv -> pre-start platform refusal (125) in 16.11.2.
$Workspace = Join-Path $ArtifactDir 'ws'
New-Item -ItemType Directory -Force -Path $Workspace | Out-Null
$rp = Start-Process -FilePath $IsolatedBinary `
    -ArgumentList 'run', '--workspace', $Workspace, '--profile', 'workspace-write', `
    '--network', 'deny', '--', 'cmd', '/C', 'exit 0' `
    -WorkingDirectory $IsolatedCwd -NoNewWindow -Wait -PassThru `
    -RedirectStandardOutput (Join-Path $ArtifactDir 'run-stdout.txt') `
    -RedirectStandardError (Join-Path $ArtifactDir 'run-stderr.txt')
Set-Content -Path (Join-Path $ArtifactDir 'run-exit.txt') -Value $rp.ExitCode -Encoding ascii
if ($rp.ExitCode -ne 125) {
    throw "opi-sandbox-smoke: expected run exit 125 (pre-start refusal), got $($rp.ExitCode)"
}
}
finally {
    Pop-Location
}

# 5. no durable state / no Opi access: canary never read; no files under sentinel.
$docRaw = Get-Content (Join-Path $ArtifactDir 'doctor.json') -Raw
if ($docRaw -match 'CANARY-opi-config-not-read') {
    throw "opi-sandbox-smoke: binary leaked sentinel config into doctor output"
}
$SentinelFiles = @(Get-ChildItem -Path $Sentinel -Recurse -File | ForEach-Object { $_.FullName } | Sort-Object)
if ($SentinelFiles.Count -ne 1 -or (Split-Path -Leaf $SentinelFiles[0]) -ne (Split-Path -Leaf $CanaryPath)) {
    throw "opi-sandbox-smoke: binary created files under sentinel: $($SentinelFiles -join ', ')"
}
$BinFiles = @(Get-ChildItem -LiteralPath $IsolatedBinDir -Force)
if ($BinFiles.Count -ne 1 -or (Split-Path -Leaf $BinFiles[0].FullName) -ne (Split-Path -Leaf $IsolatedBinary)) {
    throw "opi-sandbox-smoke: isolated bin directory gained state: $($BinFiles.FullName -join ', ')"
}
$CwdEntries = @(Get-ChildItem -LiteralPath $IsolatedCwd -Force)
if ($CwdEntries.Count -ne 0) {
    throw "opi-sandbox-smoke: isolated cwd gained state: $($CwdEntries.FullName -join ', ')"
}

Set-Content -Path (Join-Path $ArtifactDir 'smoke-result.txt') -Value 'opi-sandbox-smoke: OK' -Encoding ascii
Set-Content -Path (Join-Path $ArtifactDir 'windows-unsupported-smoke-result.txt') -Value 'opi-sandbox-windows-unsupported-smoke: OK' -Encoding ascii
Set-Content -Path (Join-Path $ArtifactDir 'windows-isolation-smoke-result.txt') -Value 'opi-sandbox-windows-isolation-smoke: OK' -Encoding ascii
exit 0
