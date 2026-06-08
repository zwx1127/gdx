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
$Work = Join-Path $env:TEMP ("gdx_existing_" + [guid]::NewGuid().ToString("N"))
$Shot = Join-Path $Work "daemon-shot.png"

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

[void](New-Item -ItemType Directory -Force -Path $Work)
Set-Content -LiteralPath (Join-Path $Work "project.godot") -Encoding UTF8 -Value "config_version=5`n`n[application]`nconfig/name=`"existing`"`n"
Invoke-Native $Bin project setup --project $Work --json
Invoke-Native $Bin project inspect --project $Work --json
Invoke-Native $Bin @Common scene new `
    --project $Work `
    --out "res://scenes/main.tscn" `
    --root-type Node2D `
    --name Main `
    --set-main `
    --json
Invoke-Native $Bin @Common asset import --project $Work --json

try {
    Invoke-Native $Bin @Common serve `
        --project $Work `
        --width 1280 `
        --height 720 `
        --restart `
        --json

    Invoke-Native $Bin status --project $Work --json
    Invoke-Native $Bin scene tree --project $Work --json
    Invoke-Native $Bin scene add-node --project $Work --parent "/" --type Label --name Subtitle --json
    Invoke-Native $Bin scene set --project $Work --node "/Subtitle" --property text --value "Edited by daemon" --json
    Invoke-Native $Bin scene set --project $Work --node "/Subtitle" --property position --vec2 40 90 --json
    Invoke-Native $Bin scene save --project $Work --json

    $SceneText = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $Work "scenes\main.tscn")
    if ($SceneText -notmatch "Subtitle") { throw "Saved scene does not include Subtitle" }
    if ($SceneText -notmatch "Edited by daemon") { throw "Saved scene does not include edited text" }

    Invoke-Native $Bin capture --project $Work --out $Shot --frames 10 --json

    $ShotInfo = Get-Item -LiteralPath $Shot
    if ($ShotInfo.Length -le 0) { throw "Capture is empty: $Shot" }
}
finally {
    & $Bin kill --project $Work --force --json
}

Write-Host "GDX MVP-1 DAEMON E2E PASS: $Shot"
