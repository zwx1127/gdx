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
$Work = Join-Path $env:TEMP ("gdx_daemon_3d_" + [guid]::NewGuid().ToString("N"))
$Shot = Join-Path $Work "daemon-3d-shot.png"

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

Invoke-Native $Bin project init --project $Work --name daemon3d --json
Invoke-Native $Bin @Common scene create `
    --project $Work `
    --out "res://scenes/main_3d.tscn" `
    --root-type Node3D `
    --name Main3D `
    --set-main `
    --json
Invoke-Native $Bin @Common asset import --project $Work --json

try {
    Invoke-Native $Bin @Common daemon start `
        --project $Work `
        --width 1280 `
        --height 720 `
        --restart `
        --json

    Invoke-Native $Bin scene tree --project $Work --json
    Invoke-Native $Bin scene node add --project $Work --parent "/" --type MeshInstance3D --name AddedBox --json

    Invoke-Native $Bin scene node set-property --project $Work --node "/AddedBox" --property position --vec3 2 0.5 0 --json
    Invoke-Native $Bin scene save --project $Work --json

    $SceneText = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $Work "scenes\main_3d.tscn")
    if ($SceneText -notmatch "AddedBox") { throw "Saved scene does not include AddedBox" }

    Invoke-Native $Bin daemon capture --project $Work --out $Shot --frames 10 --json

    $ShotInfo = Get-Item -LiteralPath $Shot
    if ($ShotInfo.Length -le 0) { throw "Capture is empty: $Shot" }
}
finally {
    & $Bin daemon stop --project $Work --force --json
}

Write-Host "GDX DAEMON 3D E2E PASS: $Shot"
