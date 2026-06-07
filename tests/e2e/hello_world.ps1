param(
    [string]$Godot = $env:GDX_GODOT
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$Root = Resolve-Path (Join-Path $PSScriptRoot "..\..")
$Bin = Join-Path $Root "target\debug\gdx.exe"
$Work = Join-Path $env:TEMP "gdx_hello"
$Shot = Join-Path $Work "shot.png"

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

Invoke-Native $Bin @Common env --json
Invoke-Native $Bin init basic --path $Work --name hello --json

if (!(Test-Path (Join-Path $Work "project.godot"))) { throw "project.godot was not created" }
if (!(Test-Path (Join-Path $Work "addons\gdx_tools\build_scene.gd"))) { throw "build_scene.gd was not created" }
if (!(Test-Path (Join-Path $Work "addons\gdx_runtime\capture_runner.gd"))) { throw "capture_runner.gd was not created" }

Invoke-Native $Bin @Common scene build `
    --project $Work `
    --spec (Join-Path $Root "examples\hello_scene.json") `
    --out "res://scenes/main.tscn" `
    --json

if (!(Test-Path (Join-Path $Work "scenes\main.tscn"))) { throw "main.tscn was not created" }

Invoke-Native $Bin @Common asset import --project $Work --json
Invoke-Native $Bin @Common code check --project $Work "res://scripts/main.gd" --json

Invoke-Native $Bin @Common play run `
    --project $Work `
    --scene "res://scenes/main.tscn" `
    --capture $Shot `
    --frames 10 `
    --width 1280 `
    --height 720 `
    --json

$ShotInfo = Get-Item -LiteralPath $Shot
if ($ShotInfo.Length -le 0) { throw "Capture is empty: $Shot" }

Write-Host "GDX MVP-0 E2E PASS: $Shot"
