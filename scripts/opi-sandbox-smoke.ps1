# Standalone acceptance smoke for the opi-sandbox binary (Phase 16 task 16.11.2).
# Launches ONLY the explicit -BinaryPath; never invokes cargo or opi.
#
# Usage: opi-sandbox-smoke.ps1 -BinaryPath PATH -ArtifactDir PATH
#
# Covers spec `### Standalone CLI acceptance` items 1-5, 8 (binary identity,
# no-opi-on-PATH, Opi-sentinel env ignored, help/version/doctor, run pre-start
# refusal, no durable state). Item 6 (installed-binary run success) and item 7
# (backend --stdio) are deferred to 16.13/16.14.1 and 16.12 respectively.
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
& $BinaryPath --help | Out-File -FilePath (Join-Path $ArtifactDir 'help.txt') -Encoding ascii
$help = Get-Content (Join-Path $ArtifactDir 'help.txt') -Raw
if (-not ($help -match 'run') -or -not ($help -match 'doctor')) {
    throw "opi-sandbox-smoke: --help missing run/doctor"
}

# 2. --version
& $BinaryPath --version | Out-File -FilePath (Join-Path $ArtifactDir 'version.txt') -Encoding ascii
if (-not ((Get-Content (Join-Path $ArtifactDir 'version.txt') -Raw) -match 'opi-sandbox')) {
    throw "opi-sandbox-smoke: --version missing opi-sandbox"
}

# 3. doctor --json (stable object; supported=false everywhere in 16.11.2)
& $BinaryPath doctor --json | Out-File -FilePath (Join-Path $ArtifactDir 'doctor.json') -Encoding ascii
$doc = Get-Content (Join-Path $ArtifactDir 'doctor.json') -Raw | ConvertFrom-Json
if ($doc.schema_version -ne 1) { throw "opi-sandbox-smoke: doctor schema_version=$($doc.schema_version)" }
if ($doc.supported -ne $false) { throw "opi-sandbox-smoke: doctor supported=$($doc.supported) (must be false in 16.11.2)" }
if ($doc.target -ne 'windows') { throw "opi-sandbox-smoke: doctor target=$($doc.target) (must be windows)" }
if (@($doc.mechanisms).Count -ne 0) { throw "opi-sandbox-smoke: doctor mechanisms not empty" }
if (-not (@($doc.profiles) -contains 'workspace-write')) { throw "opi-sandbox-smoke: doctor profiles missing workspace-write" }

# 4. run with a VALID argv -> pre-start platform refusal (125) in 16.11.2.
$Workspace = Join-Path $ArtifactDir 'ws'
New-Item -ItemType Directory -Force -Path $Workspace | Out-Null
$rp = Start-Process -FilePath $BinaryPath `
    -ArgumentList 'run', '--workspace', $Workspace, '--profile', 'workspace-write', `
    '--network', 'deny', '--', 'cmd', '/C', 'exit 0' `
    -NoNewWindow -Wait -PassThru `
    -RedirectStandardOutput (Join-Path $ArtifactDir 'run-stdout.txt') `
    -RedirectStandardError (Join-Path $ArtifactDir 'run-stderr.txt')
Set-Content -Path (Join-Path $ArtifactDir 'run-exit.txt') -Value $rp.ExitCode -Encoding ascii
if ($rp.ExitCode -ne 125) {
    throw "opi-sandbox-smoke: expected run exit 125 (pre-start refusal), got $($rp.ExitCode)"
}

# 5. no durable state / no Opi access: canary never read; no files under sentinel.
$docRaw = Get-Content (Join-Path $ArtifactDir 'doctor.json') -Raw
if ($docRaw -match 'CANARY-opi-config-not-read') {
    throw "opi-sandbox-smoke: binary leaked sentinel config into doctor output"
}
$SentinelFiles = @(Get-ChildItem -Path $Sentinel -Recurse -File | ForEach-Object { $_.FullName } | Sort-Object)
if ($SentinelFiles.Count -ne 1 -or $SentinelFiles[0] -ne $CanaryPath) {
    throw "opi-sandbox-smoke: binary created files under sentinel: $($SentinelFiles -join ', ')"
}

Set-Content -Path (Join-Path $ArtifactDir 'smoke-result.txt') -Value 'opi-sandbox-smoke: OK' -Encoding ascii
exit 0
