$ErrorActionPreference = "Stop"

$guard = Join-Path $PSScriptRoot "ledger-guard.ps1"
$tempRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("opi-ledger-guard-" + [guid]::NewGuid().ToString("N"))
$utf8 = New-Object System.Text.UTF8Encoding($false, $true)

function Write-Utf8File {
    param(
        [Parameter(Mandatory = $true)][string]$Path,
        [Parameter(Mandatory = $true)][string]$Text
    )

    [System.IO.File]::WriteAllText($Path, $Text, $utf8)
}

function Read-Utf8File {
    param([Parameter(Mandatory = $true)][string]$Path)

    return [System.IO.File]::ReadAllText($Path, $utf8)
}

function Invoke-Guard {
    param([Parameter(Mandatory = $true)][string[]]$Arguments)

    $previousErrorAction = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    try {
        $output = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $guard @Arguments 2>&1
    } finally {
        $ErrorActionPreference = $previousErrorAction
    }
    return [pscustomobject]@{
        ExitCode = $LASTEXITCODE
        Output = ($output -join [Environment]::NewLine)
    }
}

function Assert-Equal {
    param(
        [Parameter(Mandatory = $true)]$Expected,
        [Parameter(Mandatory = $true)]$Actual,
        [Parameter(Mandatory = $true)][string]$Message
    )

    if ($Expected -ne $Actual) {
        throw "$Message. Expected '$Expected', got '$Actual'."
    }
}

function Assert-Contains {
    param(
        [Parameter(Mandatory = $true)][string]$Needle,
        [Parameter(Mandatory = $true)][string]$Haystack,
        [Parameter(Mandatory = $true)][string]$Message
    )

    if (-not $Haystack.Contains($Needle)) {
        throw "$Message. Missing '$Needle' in '$Haystack'."
    }
}

try {
    if (-not (Test-Path -LiteralPath $guard)) {
        throw "ledger-guard.ps1 is missing"
    }

    [System.IO.Directory]::CreateDirectory($tempRoot) | Out-Null

    $dash = [string][char]0x2014
    $section = [string][char]0x00A7
    $validPath = Join-Path $tempRoot "valid.json"
    Write-Utf8File $validPath ('{"schema_version":2,"message":"valid ' + $dash + ' ' + $section + '"}')
    $valid = Invoke-Guard @("-Command", "Validate", "-Path", $validPath)
    Assert-Equal 0 $valid.ExitCode "Valid UTF-8 ledger should pass"

    $validChinesePath = Join-Path $tempRoot "valid-chinese.json"
    $validChinese = ([string][char]0x6402) + ([string][char]0x62B1)
    Write-Utf8File $validChinesePath ('{"schema_version":2,"message":"' + $validChinese + '"}')
    $validChineseResult = Invoke-Guard @("-Command", "Validate", "-Path", $validChinesePath)
    Assert-Equal 0 $validChineseResult.ExitCode "Legitimate Chinese text should pass"

    $mojibakePath = Join-Path $tempRoot "mojibake.json"
    $mojibake = ([string][char]0x95C2) + "?"
    Write-Utf8File $mojibakePath ('{"schema_version":2,"message":"' + $mojibake + '"}')
    $badEncoding = Invoke-Guard @("-Command", "Validate", "-Path", $mojibakePath)
    Assert-Equal 1 $badEncoding.ExitCode "Known mojibake should be rejected"
    Assert-Contains "mojibake" $badEncoding.Output "Mojibake failure should be explicit"

    $sectionMojibakePath = Join-Path $tempRoot "section-mojibake.json"
    $sectionMojibake = [string][char]0x6402
    Write-Utf8File $sectionMojibakePath ('{"schema_version":2,"message":"' + $sectionMojibake + '5.3"}')
    $badSection = Invoke-Guard @("-Command", "Validate", "-Path", $sectionMojibakePath)
    Assert-Equal 1 $badSection.ExitCode "Mojibake section marker should be rejected"
    Assert-Contains "mojibake" $badSection.Output "Section-marker failure should be explicit"

    $arrowMojibakePath = Join-Path $tempRoot "arrow-mojibake.json"
    $arrowMojibake = ([string][char]0x922B) + "?"
    Write-Utf8File $arrowMojibakePath ('{"schema_version":2,"message":"' + $arrowMojibake + '"}')
    $badArrow = Invoke-Guard @("-Command", "Validate", "-Path", $arrowMojibakePath)
    Assert-Equal 1 $badArrow.ExitCode "Mojibake arrow marker should be rejected"
    Assert-Contains "mojibake" $badArrow.Output "Arrow-marker failure should be explicit"

    $largePath = Join-Path $tempRoot "large.json"
    Write-Utf8File $largePath ('{"schema_version":2,"message":"' + ("x" * 128) + '"}')
    $tooLarge = Invoke-Guard @("-Command", "Validate", "-Path", $largePath, "-MaxBytes", "64")
    Assert-Equal 1 $tooLarge.ExitCode "Oversized ledger should be rejected"
    Assert-Contains "size limit" $tooLarge.Output "Size failure should be explicit"

    $longStringPath = Join-Path $tempRoot "long-string.json"
    Write-Utf8File $longStringPath ('{"schema_version":2,"message":"' + ("y" * 64) + '"}')
    $longString = Invoke-Guard @("-Command", "Validate", "-Path", $longStringPath, "-MaxStringChars", "32")
    Assert-Equal 1 $longString.ExitCode "Oversized string should be rejected"
    Assert-Contains "string limit" $longString.Output "String failure should be explicit"

    $targetPath = Join-Path $tempRoot "target.json"
    $candidatePath = Join-Path $tempRoot "candidate.json"
    $backupPath = Join-Path $tempRoot "target.corrupt.json"
    Write-Utf8File $targetPath '{"schema_version":2,"value":"old"}'
    $replacementValue = [string][char]0x6B63
    Write-Utf8File $candidatePath ('{"schema_version":2,"value":"' + $replacementValue + '"}')
    $expectedHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $targetPath).Hash
    $installed = Invoke-Guard @(
        "-Command", "Install",
        "-Path", $candidatePath,
        "-TargetPath", $targetPath,
        "-ExpectedTargetSha256", $expectedHash,
        "-BackupPath", $backupPath
    )
    Assert-Equal 0 $installed.ExitCode ("Validated candidate should install: " + $installed.Output)
    Assert-Contains $replacementValue (Read-Utf8File $targetPath) "Install should preserve Unicode"
    Assert-Contains "old" (Read-Utf8File $backupPath) "Install should retain requested backup"

    $beforeStaleInstall = Read-Utf8File $targetPath
    Write-Utf8File $candidatePath '{"schema_version":2,"value":"newer"}'
    $stale = Invoke-Guard @(
        "-Command", "Install",
        "-Path", $candidatePath,
        "-TargetPath", $targetPath,
        "-ExpectedTargetSha256", ("0" * 64)
    )
    Assert-Equal 1 $stale.ExitCode "Concurrent target change should reject install"
    Assert-Contains "changed since inspection" $stale.Output "Concurrent-write failure should be explicit"
    Assert-Equal $beforeStaleInstall (Read-Utf8File $targetPath) "Rejected install must preserve target"

    Write-Utf8File $candidatePath '{"schema_version":2,"value":"unterminated}'
    $invalid = Invoke-Guard @(
        "-Command", "Install",
        "-Path", $candidatePath,
        "-TargetPath", $targetPath,
        "-ExpectedTargetSha256", (Get-FileHash -Algorithm SHA256 -LiteralPath $targetPath).Hash
    )
    Assert-Equal 1 $invalid.ExitCode "Invalid JSON candidate should reject install"
    Assert-Equal $beforeStaleInstall (Read-Utf8File $targetPath) "Invalid candidate must preserve target"

    Write-Output "ledger-guard tests passed"
} finally {
    if (Test-Path -LiteralPath $tempRoot) {
        $resolvedTempRoot = [System.IO.Path]::GetFullPath($tempRoot)
        $systemTempRoot = [System.IO.Path]::GetFullPath([System.IO.Path]::GetTempPath())
        $tempLeaf = [System.IO.Path]::GetFileName($resolvedTempRoot)
        if (-not $resolvedTempRoot.StartsWith($systemTempRoot, [System.StringComparison]::OrdinalIgnoreCase) -or
            -not $tempLeaf.StartsWith("opi-ledger-guard-", [System.StringComparison]::Ordinal)) {
            throw "refusing to remove unexpected test path: $resolvedTempRoot"
        }
        Remove-Item -LiteralPath $tempRoot -Recurse -Force
    }
}
