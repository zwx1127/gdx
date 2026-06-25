param(
    [string]$Godot = $env:GDX_GODOT
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$Root = Resolve-Path (Join-Path $PSScriptRoot "..\..")
$Bin = Join-Path $Root "target\debug\gdx.exe"
$Work = Join-Path $env:TEMP ("gdx_record_" + [guid]::NewGuid().ToString("N"))
$Movie = Join-Path $Work "recording.avi"

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

Invoke-Native $Bin project create --path $Work --name record
Invoke-Native $Bin @Common --project $Work scene create `
    --out "res://scenes/main.tscn" `
    --root-type Node2D `
    --name Main `
    --set-main

$ScriptsDir = Join-Path $Work "scripts"
[void](New-Item -ItemType Directory -Force -Path $ScriptsDir)
$ScriptPath = Join-Path $ScriptsDir "recording_demo.gd"
$ScriptText = @'
extends Node2D

var t: float = 0.0

func _process(delta: float) -> void:
    t += delta
    queue_redraw()

func _draw() -> void:
    var x := 40.0 + sin(t * 4.0) * 24.0
    draw_rect(Rect2(x, 48.0, 96.0, 72.0), Color(0.1, 0.65, 1.0, 1.0))
    draw_circle(Vector2(190.0, 120.0), 28.0 + sin(t * 5.0) * 8.0, Color(1.0, 0.55, 0.15, 1.0))
'@
[System.IO.File]::WriteAllText($ScriptPath, $ScriptText, [System.Text.UTF8Encoding]::new($false))

Invoke-Native $Bin @Common --project $Work script attach `
    --scene "res://scenes/main.tscn" `
    --node "/" `
    --script "res://scripts/recording_demo.gd"
Invoke-Native $Bin @Common --project $Work script check-all
Invoke-Native $Bin @Common --project $Work capture record `
    --scene "res://scenes/main.tscn" `
    --out $Movie `
    --duration 1 `
    --fps 12 `
    --width 320 `
    --height 240

$MovieInfo = Get-Item -LiteralPath $Movie
if ($MovieInfo.Length -le 0) { throw "Recording is empty: $Movie" }

$Header = [System.IO.File]::ReadAllBytes($Movie)[0..11]
$HeaderText = [System.Text.Encoding]::ASCII.GetString($Header)
if (!$HeaderText.StartsWith("RIFF") -or !$HeaderText.Contains("AVI ")) {
    throw "Recording is not an AVI file: $Movie"
}

Write-Host "GDX RECORD MOVIE E2E PASS: $Movie"
