param(
    [string]$Godot = $env:GDX_GODOT
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$Dotnet = "C:\Program Files\dotnet"
if (Test-Path (Join-Path $Dotnet "dotnet.exe")) {
    $env:Path = "$Dotnet;$env:Path"
}

$Root = Resolve-Path (Join-Path $PSScriptRoot "..\..")
$Bin = Join-Path $Root "target\debug\gdx.exe"
$Work = Join-Path $env:TEMP "gdx_hello_3d"
$Shot = Join-Path $Work "shot-3d.png"

function Invoke-Native {
    param(
        [Parameter(Mandatory = $true)]
        [string]$FilePath,

        [Parameter(ValueFromRemainingArguments = $true)]
        [string[]]$Arguments
    )

    & $FilePath @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "Command failed with exit code ${LASTEXITCODE}: $FilePath $($Arguments -join ' ')"
    }
}

if ([string]::IsNullOrWhiteSpace($Godot)) {
    throw "Set GDX_GODOT or pass -Godot with a Godot 4.x executable path."
}

if (Test-Path $Work) {
    Remove-Item -LiteralPath $Work -Recurse -Force
}

Invoke-Native cargo build -p gdx-cli

$Common = @("--godot", $Godot)

Invoke-Native $Bin init basic --path $Work --name hello3d --json
Invoke-Native $Bin @Common scene build `
    --project $Work `
    --spec (Join-Path $Root "examples\hello_3d_scene.json") `
    --out "res://scenes/main_3d.tscn" `
    --json

if (!(Test-Path (Join-Path $Work "scenes\main_3d.tscn"))) { throw "main_3d.tscn was not created" }

Invoke-Native $Bin @Common asset import --project $Work --json

Invoke-Native $Bin @Common play run `
    --project $Work `
    --scene "res://scenes/main_3d.tscn" `
    --capture $Shot `
    --frames 10 `
    --width 1280 `
    --height 720 `
    --json

$ShotInfo = Get-Item -LiteralPath $Shot
if ($ShotInfo.Length -le 0) { throw "Capture is empty: $Shot" }

Write-Host "GDX BASIC 3D E2E PASS: $Shot"
