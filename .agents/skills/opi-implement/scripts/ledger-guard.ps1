param(
    [Parameter(Mandatory = $true)]
    [ValidateSet("Validate", "Install")]
    [string]$Command,

    [Parameter(Mandatory = $true)]
    [string]$Path,

    [string]$TargetPath,

    [string]$BackupPath,

    [string]$ExpectedTargetSha256,

    [long]$MaxBytes = 16777216,

    [int]$MaxStringChars = 65536
)

$ErrorActionPreference = "Stop"
$strictUtf8 = New-Object System.Text.UTF8Encoding($false, $true)
$sensitivePropertyNames = @(
    "api_key",
    "apikey",
    "access_token",
    "refresh_token",
    "authorization",
    "password",
    "private_key",
    "client_secret"
)

function Assert-LedgerPropertyNameSafe {
    param(
        [Parameter(Mandatory = $true)][string]$PropertyName,
        [Parameter(Mandatory = $true)][string]$JsonPath
    )

    if ($sensitivePropertyNames -contains $PropertyName) {
        throw "ledger sensitive content detected at ${JsonPath}: forbidden property '$PropertyName'"
    }
}

function Get-Sha256 {
    param([Parameter(Mandatory = $true)][string]$FilePath)

    $stream = [System.IO.File]::OpenRead($FilePath)
    $sha = [System.Security.Cryptography.SHA256]::Create()
    try {
        $hash = $sha.ComputeHash($stream)
        return (($hash | ForEach-Object { $_.ToString("x2") }) -join "")
    } finally {
        $sha.Dispose()
        $stream.Dispose()
    }
}

function Test-LedgerValue {
    param(
        $Value,
        [Parameter(Mandatory = $true)][string]$JsonPath,
        [Parameter(Mandatory = $true)][int]$StringLimit
    )

    if ($null -eq $Value) {
        return
    }

    if ($Value -is [string]) {
        if ($Value.Length -gt $StringLimit) {
            throw "ledger string limit exceeded at ${JsonPath}: $($Value.Length) > $StringLimit"
        }

        $bearerPattern = "(?i)\bBearer\s+(?<token>[-A-Za-z0-9._~+/]{12,}=*)"
        foreach ($match in [System.Text.RegularExpressions.Regex]::Matches($Value, $bearerPattern)) {
            $token = $match.Groups["token"].Value
            $isExplicitPlaceholder = [System.Text.RegularExpressions.Regex]::IsMatch(
                $token,
                "^[a-z]+(?:[-_][a-z]+)*[-_]token$"
            )
            if (-not $isExplicitPlaceholder) {
                throw "ledger sensitive content detected at $JsonPath"
            }
        }
        if ([System.Text.RegularExpressions.Regex]::IsMatch(
            $Value,
            "-----BEGIN(?: [A-Z0-9]+)? PRIVATE KEY-----"
        )) {
            throw "ledger sensitive content detected at $JsonPath"
        }

        $markers = @(
            (([string][char]0x9225) + "?"),
            (([string][char]0x922B) + "?"),
            (([string][char]0x95B3) + "?"),
            (([string][char]0x95C1) + "?"),
            (([string][char]0x95C2) + "?"),
            ([string][char]0xFFFD)
        )
        foreach ($marker in $markers) {
            if ($Value.Contains($marker)) {
                throw "ledger mojibake detected at $JsonPath"
            }
        }
        $sectionMarkerPattern = [System.Text.RegularExpressions.Regex]::Escape([string][char]0x6402) + "\s*[0-9]"
        if ([System.Text.RegularExpressions.Regex]::IsMatch($Value, $sectionMarkerPattern)) {
            throw "ledger mojibake detected at $JsonPath"
        }
        return
    }

    if ($Value -is [System.Collections.IDictionary]) {
        foreach ($key in $Value.Keys) {
            Assert-LedgerPropertyNameSafe -PropertyName ([string]$key) -JsonPath "$JsonPath.$key"
            Test-LedgerValue -Value $Value[$key] -JsonPath "$JsonPath.$key" -StringLimit $StringLimit
        }
        return
    }

    if (($Value -is [System.Collections.IEnumerable]) -and -not ($Value -is [string])) {
        $index = 0
        foreach ($item in $Value) {
            Test-LedgerValue -Value $item -JsonPath "$JsonPath[$index]" -StringLimit $StringLimit
            $index += 1
        }
        return
    }

    foreach ($property in $Value.PSObject.Properties) {
        Assert-LedgerPropertyNameSafe -PropertyName $property.Name -JsonPath "$JsonPath.$($property.Name)"
        Test-LedgerValue -Value $property.Value -JsonPath "$JsonPath.$($property.Name)" -StringLimit $StringLimit
    }
}

function Read-AndValidateLedger {
    param(
        [Parameter(Mandatory = $true)][string]$LedgerPath,
        [Parameter(Mandatory = $true)][long]$ByteLimit,
        [Parameter(Mandatory = $true)][int]$StringLimit
    )

    if (-not (Test-Path -LiteralPath $LedgerPath -PathType Leaf)) {
        throw "ledger file not found: $LedgerPath"
    }

    $resolved = (Resolve-Path -LiteralPath $LedgerPath).Path
    $bytes = [System.IO.File]::ReadAllBytes($resolved)
    if ($bytes.LongLength -gt $ByteLimit) {
        throw "ledger size limit exceeded: $($bytes.LongLength) > $ByteLimit"
    }

    if ($bytes.Length -ge 3 -and $bytes[0] -eq 0xEF -and $bytes[1] -eq 0xBB -and $bytes[2] -eq 0xBF) {
        throw "ledger must be UTF-8 without BOM"
    }

    try {
        $text = $strictUtf8.GetString($bytes)
    } catch {
        throw "ledger is not strict UTF-8: $($_.Exception.Message)"
    }

    try {
        if ((Get-Command ConvertFrom-Json).Parameters.ContainsKey("DateKind")) {
            $ledger = $text | ConvertFrom-Json -DateKind String
        } else {
            $ledger = $text | ConvertFrom-Json
        }
    } catch {
        throw "ledger is not valid JSON: $($_.Exception.Message)"
    }

    if ($null -eq $ledger -or $null -eq $ledger.PSObject.Properties["schema_version"]) {
        throw "ledger schema_version is missing"
    }
    if ([int]$ledger.schema_version -ne 2) {
        throw "ledger schema_version must be 2"
    }

    Test-LedgerValue -Value $ledger -JsonPath '$' -StringLimit $StringLimit

    return [pscustomobject]@{
        Bytes = $bytes
        Length = $bytes.LongLength
        Sha256 = Get-Sha256 -FilePath $resolved
    }
}

function Assert-ExpectedTargetHash {
    param(
        [Parameter(Mandatory = $true)][string]$LedgerTargetPath,
        [string]$ExpectedHash
    )

    if ([string]::IsNullOrWhiteSpace($ExpectedHash)) {
        return
    }
    if (-not (Test-Path -LiteralPath $LedgerTargetPath -PathType Leaf)) {
        throw "target ledger changed since inspection: file is missing"
    }

    $actual = Get-Sha256 -FilePath $LedgerTargetPath
    if (-not $actual.Equals($ExpectedHash, [System.StringComparison]::OrdinalIgnoreCase)) {
        throw "target ledger changed since inspection: expected $ExpectedHash, got $actual"
    }
}

try {
    $candidate = Read-AndValidateLedger -LedgerPath $Path -ByteLimit $MaxBytes -StringLimit $MaxStringChars

    if ($Command -eq "Validate") {
        Write-Output "ledger validation ok bytes=$($candidate.Length) sha256=$($candidate.Sha256)"
        exit 0
    }

    if ([string]::IsNullOrWhiteSpace($TargetPath)) {
        throw "TargetPath is required for Install"
    }

    $targetFullPath = [System.IO.Path]::GetFullPath($TargetPath)
    $targetDirectory = [System.IO.Path]::GetDirectoryName($targetFullPath)
    if (-not (Test-Path -LiteralPath $targetDirectory -PathType Container)) {
        throw "target directory not found: $targetDirectory"
    }

    $requestedBackupPath = $null
    if (-not [string]::IsNullOrWhiteSpace($BackupPath)) {
        $requestedBackupPath = [System.IO.Path]::GetFullPath($BackupPath)
        $backupDirectory = [System.IO.Path]::GetDirectoryName($requestedBackupPath)
        if (-not $backupDirectory.Equals($targetDirectory, [System.StringComparison]::OrdinalIgnoreCase)) {
            throw "BackupPath must be in the target ledger directory"
        }
        if (Test-Path -LiteralPath $requestedBackupPath) {
            throw "ledger backup already exists: $requestedBackupPath"
        }
        if (-not (Test-Path -LiteralPath $targetFullPath -PathType Leaf)) {
            throw "BackupPath requires an existing target ledger"
        }
    }

    Assert-ExpectedTargetHash -LedgerTargetPath $targetFullPath -ExpectedHash $ExpectedTargetSha256

    $tempPath = $targetFullPath + ".tmp"
    if (Test-Path -LiteralPath $tempPath) {
        throw "ledger temp path already exists: $tempPath"
    }

    $createdTemp = $false
    try {
        $options = [System.IO.FileOptions]::WriteThrough
        $stream = New-Object System.IO.FileStream(
            $tempPath,
            [System.IO.FileMode]::CreateNew,
            [System.IO.FileAccess]::Write,
            [System.IO.FileShare]::None,
            4096,
            $options
        )
        $createdTemp = $true
        try {
            $stream.Write($candidate.Bytes, 0, $candidate.Bytes.Length)
            $stream.Flush($true)
        } finally {
            $stream.Dispose()
        }

        $null = Read-AndValidateLedger -LedgerPath $tempPath -ByteLimit $MaxBytes -StringLimit $MaxStringChars
        Assert-ExpectedTargetHash -LedgerTargetPath $targetFullPath -ExpectedHash $ExpectedTargetSha256

        if (Test-Path -LiteralPath $targetFullPath -PathType Leaf) {
            $replacementBackupPath = $requestedBackupPath
            $deleteReplacementBackup = $false
            if ($null -eq $replacementBackupPath) {
                $replacementBackupPath = $targetFullPath + ".replace-backup"
                $deleteReplacementBackup = $true
            }
            if (Test-Path -LiteralPath $replacementBackupPath) {
                throw "ledger replacement backup already exists: $replacementBackupPath"
            }
            [System.IO.File]::Replace($tempPath, $targetFullPath, $replacementBackupPath, $true)
            if ($deleteReplacementBackup) {
                Remove-Item -LiteralPath $replacementBackupPath -Force
            }
        } else {
            [System.IO.File]::Move($tempPath, $targetFullPath)
        }
        $createdTemp = $false
    } finally {
        if ($createdTemp -and (Test-Path -LiteralPath $tempPath)) {
            Remove-Item -LiteralPath $tempPath -Force
        }
    }

    $installedHash = Get-Sha256 -FilePath $targetFullPath
    Write-Output "ledger install ok sha256=$installedHash"
    exit 0
} catch {
    [Console]::Error.WriteLine($_.Exception.Message)
    exit 1
}
