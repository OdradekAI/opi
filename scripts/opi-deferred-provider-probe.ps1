<#!
.SYNOPSIS
  Probe the real OpenAI Responses Background submit/fetch/cancel contract.

.DESCRIPTION
  This probe is deliberately live-service only. It never substitutes a
  wiremock, Faux, or scripted backend for the provider. The receipt records
  only redacted status facts and never writes the API key or full response
  bodies. A missing credential exits with code 2 and produces a blocked
  receipt, so callers cannot mistake a local contract check for live evidence.
#>
[CmdletBinding()]
param(
    [string]$ApiKeyEnv = "OPENAI_API_KEY",
    [string]$BaseUrl = "https://api.openai.com/v1",
    [string]$Model = "o3",
    [string]$InputText = "opi #143 deferred provider probe",
    [string]$ReceiptPath
)

$ErrorActionPreference = "Stop"

function Write-Receipt {
    param(
        [hashtable]$Receipt,
        [int]$ExitCode
    )

    $json = $Receipt | ConvertTo-Json -Depth 12
    if ($ReceiptPath) {
        $parent = Split-Path -Parent $ReceiptPath
        if ($parent) {
            New-Item -ItemType Directory -Force -Path $parent | Out-Null
        }
        Set-Content -LiteralPath $ReceiptPath -Value $json -Encoding UTF8
    }
    Write-Output $json
    exit $ExitCode
}

function Get-RedactedResponse {
    param(
        [int]$StatusCode,
        [string]$Body,
        [string]$ErrorMessage
    )

    $result = [ordered]@{ status = $StatusCode }
    if ($ErrorMessage) {
        $result.transportError = $ErrorMessage.Substring(0, [Math]::Min(240, $ErrorMessage.Length))
    }
    if (-not $Body) {
        return $result
    }
    try {
        $value = $Body | ConvertFrom-Json
        foreach ($name in @("id", "object", "status", "model", "created_at", "completed_at", "failed_at", "cancelled_at", "expires_at")) {
            $property = $value.PSObject.Properties[$name]
            if ($null -ne $property) {
                $result[$name] = $property.Value
            }
        }
        if ($value.error) {
            $errorValue = $value.error
            $errorRecord = [ordered]@{}
            foreach ($name in @("type", "code", "param")) {
                $property = $errorValue.PSObject.Properties[$name]
                if ($null -ne $property) {
                    $errorRecord[$name] = $property.Value
                }
            }
            if ($errorValue.message) {
                $message = [string]$errorValue.message
                $errorRecord.message = $message.Substring(0, [Math]::Min(240, $message.Length))
            }
            $result.error = $errorRecord
        }
    } catch {
        $result.bodyShape = "non-json"
    }
    return $result
}

function Invoke-ProviderJson {
    param(
        [ValidateSet("GET", "POST")]
        [string]$Method,
        [string]$Uri,
        [string]$Key,
        [string]$Body
    )

    $headers = @{ Authorization = "Bearer $Key" }
    try {
        if ($Method -eq "GET") {
            $response = Invoke-WebRequest -Method Get -Uri $Uri -Headers $headers -UseBasicParsing
        } else {
            $response = Invoke-WebRequest -Method Post -Uri $Uri -Headers $headers -ContentType "application/json" -Body $Body -UseBasicParsing
        }
        return [ordered]@{
            ok = $true
            status = [int]$response.StatusCode
            response = Get-RedactedResponse -StatusCode ([int]$response.StatusCode) -Body ([string]$response.Content)
        }
    } catch {
        $statusCode = 0
        $body = ""
        if ($_.Exception.Response) {
            $statusCode = [int]$_.Exception.Response.StatusCode
            try {
                $reader = New-Object IO.StreamReader($_.Exception.Response.GetResponseStream())
                $body = $reader.ReadToEnd()
                $reader.Dispose()
            } catch {
                $body = ""
            }
        }
        return [ordered]@{
            ok = $false
            status = $statusCode
            response = Get-RedactedResponse -StatusCode $statusCode -Body $body -ErrorMessage $_.Exception.Message
        }
    }
}

$key = [Environment]::GetEnvironmentVariable($ApiKeyEnv)
$timestamp = (Get-Date).ToUniversalTime().ToString("o")
$endpoint = $BaseUrl.TrimEnd("/")
$receipt = [ordered]@{
    schemaVersion = 1
    issue = "#143"
    observedAt = $timestamp
    provider = "openai"
    api = "openai-responses"
    apiRevision = "Responses API background mode"
    model = $Model
    endpoint = $endpoint
    auth = [ordered]@{ source = "env $ApiKeyEnv"; available = $false }
    contract = [ordered]@{
        submit = "POST /responses with background=true and store=true"
        fetch = "GET /responses/{response_id}"
        cancel = "POST /responses/{response_id}/cancel"
    }
    operations = [ordered]@{}
    liveEvidence = $false
}

if ([string]::IsNullOrWhiteSpace($key)) {
    $receipt.operations.submit = [ordered]@{ status = "blocked"; reason = "credential unavailable" }
    $receipt.operations.fetch = [ordered]@{ status = "blocked"; reason = "submit did not produce a response id" }
    $receipt.operations.cancel = [ordered]@{ status = "blocked"; reason = "submit did not produce a response id" }
    $receipt.blockers = @("$ApiKeyEnv is not set; no live request was sent")
    Write-Receipt -Receipt $receipt -ExitCode 2
}

$receipt.auth.available = $true
$submitBody = @{
    model = $Model
    input = $InputText
    background = $true
    store = $true
} | ConvertTo-Json -Compress

$submit = Invoke-ProviderJson -Method POST -Uri "$endpoint/responses" -Key $key -Body $submitBody
$receipt.operations.submit = $submit.response
$responseId = $submit.response.id
if (-not $submit.ok -or [string]::IsNullOrWhiteSpace([string]$responseId)) {
    $receipt.blockers = @("submit did not return a response id")
    Write-Receipt -Receipt $receipt -ExitCode 1
}

$fetch = Invoke-ProviderJson -Method GET -Uri "$endpoint/responses/$responseId" -Key $key
$receipt.operations.fetch = $fetch.response

$cancel = Invoke-ProviderJson -Method POST -Uri "$endpoint/responses/$responseId/cancel" -Key $key -Body "{}"
$receipt.operations.cancel = $cancel.response

$receipt.liveEvidence = $submit.ok -and $fetch.ok -and $cancel.ok
if (-not $receipt.liveEvidence) {
    $receipt.blockers = @("one or more live operations failed; inspect redacted status/error fields")
    Write-Receipt -Receipt $receipt -ExitCode 1
}

Write-Receipt -Receipt $receipt -ExitCode 0
