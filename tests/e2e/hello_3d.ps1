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
$Work = Join-Path $env:TEMP ("gdx_e2e_3d_" + [guid]::NewGuid().ToString("N"))
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

Invoke-Native $Bin init --path $Work --name hello3d --json
Invoke-Native $Bin @Common scene new `
    --project $Work `
    --out "res://scenes/main_3d.tscn" `
    --root-type Node3D `
    --name Main3D `
    --set-main `
    --json

if (!(Test-Path (Join-Path $Work "scenes\main_3d.tscn"))) { throw "main_3d.tscn was not created" }

Invoke-Native $Bin @Common asset import --project $Work --json

try {
    Invoke-Native $Bin @Common serve --project $Work --restart --json
    Invoke-Native $Bin scene add-node --project $Work --parent "/" --type Camera3D --name Camera --json
    Invoke-Native $Bin scene set --project $Work --node "/Camera" --property position --vec3 0 3 6 --json
    Invoke-Native $Bin scene set --project $Work --node "/Camera" --property rotation_degrees --vec3 -25 0 0 --json
    Invoke-Native $Bin scene set --project $Work --node "/Camera" --property current --bool true --json
    Invoke-Native $Bin scene add-node --project $Work --parent "/" --type DirectionalLight3D --name Sun --json
    Invoke-Native $Bin scene set --project $Work --node "/Sun" --property rotation_degrees --vec3 -45 -30 0 --json
    Invoke-Native $Bin scene add-node --project $Work --parent "/" --type MeshInstance3D --name Cube --json
    Invoke-Native $Bin scene set --project $Work --node "/Cube" --property position --vec3 0 0.5 0 --json
    Invoke-Native $Bin scene save --project $Work --json
    Invoke-Native $Bin capture --project $Work --out $Shot --frames 10 --json
}
finally {
    & $Bin kill --project $Work --force --json
}

$ShotInfo = Get-Item -LiteralPath $Shot
if ($ShotInfo.Length -le 0) { throw "Capture is empty: $Shot" }

Write-Host "GDX BASIC 3D E2E PASS: $Shot"
