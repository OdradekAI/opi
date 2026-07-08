param(
    [Parameter(Mandatory = $true)]
    [string]$ArtifactDir,
    [string]$WorkspaceRoot = "",
    [switch]$Json
)

$ErrorActionPreference = "Stop"
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$PythonScript = Join-Path $ScriptDir "opi-artifact-audit.py"
$ArgsList = @($PythonScript, $ArtifactDir)
if ($WorkspaceRoot -ne "") {
    $ArgsList += @("--workspace-root", $WorkspaceRoot)
}
if ($Json) {
    $ArgsList += "--json"
}
python @ArgsList
exit $LASTEXITCODE
