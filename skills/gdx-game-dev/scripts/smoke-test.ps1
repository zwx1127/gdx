param(
    [string]$RepoRoot,
    [string]$Project,
    [string]$Godot
)

$ErrorActionPreference = "Stop"

$scriptDir = Split-Path -Parent $PSCommandPath
$gdx = & (Join-Path $scriptDir "resolve-gdx.ps1") -RepoRoot $RepoRoot

$globalArgs = @()
if ($Godot) {
    $globalArgs += @("--godot", $Godot)
}

& $gdx @globalArgs doctor
& $gdx @globalArgs --help | Out-Null

if ($Project) {
    & $gdx @globalArgs --project $Project project inspect
}

Write-Output (@{
    ok = $true
    gdx = $gdx
    project = $Project
} | ConvertTo-Json -Compress)
