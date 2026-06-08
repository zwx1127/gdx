param(
    [string]$Godot = $env:GDX_GODOT
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$Root = Resolve-Path (Join-Path $PSScriptRoot "..\..")
$Bin = Join-Path $Root "target\debug\gdx.exe"
$Work = Join-Path $env:TEMP ("gdx_e2e_" + [guid]::NewGuid().ToString("N"))
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
Invoke-Native $Bin init --path $Work --name hello --json

if (!(Test-Path (Join-Path $Work "project.godot"))) { throw "project.godot was not created" }
if (!(Test-Path (Join-Path $Work "addons\gdx_tools\create_scene.gd"))) { throw "create_scene.gd was not created" }
if (!(Test-Path (Join-Path $Work "addons\gdx_runtime\capture_runner.gd"))) { throw "capture_runner.gd was not created" }
if (Test-Path (Join-Path $Work "scripts\main.gd")) { throw "scripts\main.gd should not be created by init" }

Invoke-Native $Bin @Common scene new `
    --project $Work `
    --out "res://scenes/main.tscn" `
    --root-type Node2D `
    --name Main `
    --set-main `
    --json

if (!(Test-Path (Join-Path $Work "scenes\main.tscn"))) { throw "main.tscn was not created" }

Invoke-Native $Bin @Common asset import --project $Work --json

try {
    Invoke-Native $Bin @Common serve --project $Work --restart --json
    Invoke-Native $Bin scene add-node --project $Work --parent "/" --type Label --name Title --json
    Invoke-Native $Bin scene set --project $Work --node "/Title" --property text --value "Hello gdx" --json
    Invoke-Native $Bin scene set --project $Work --node "/Title" --property position --vec2 40 40 --json
    Invoke-Native $Bin scene save --project $Work --json
    Invoke-Native $Bin capture --project $Work --out $Shot --frames 10 --json
}
finally {
    & $Bin kill --project $Work --force --json
}

$ShotInfo = Get-Item -LiteralPath $Shot
if ($ShotInfo.Length -le 0) { throw "Capture is empty: $Shot" }

Write-Host "GDX MVP-0 E2E PASS: $Shot"
