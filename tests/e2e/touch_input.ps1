param(
    [string]$Godot = $env:GDX_GODOT
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$Root = Resolve-Path (Join-Path $PSScriptRoot "..\..")
$Bin = Join-Path $Root "target\debug\gdx.exe"
$Work = Join-Path $env:TEMP ("gdx_touch_input_" + [guid]::NewGuid().ToString("N"))

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

function Invoke-Json {
    param(
        [Parameter(Mandatory = $true)]
        [string]$FilePath,

        [Parameter(ValueFromRemainingArguments = $true)]
        [string[]]$Arguments
    )

    $Output = & $FilePath @Arguments | Out-String
    if ($LASTEXITCODE -ne 0) {
        throw "Command failed with exit code ${LASTEXITCODE}: $FilePath $($Arguments -join ' ')`n$Output"
    }
    return $Output | ConvertFrom-Json
}

if ([string]::IsNullOrWhiteSpace($Godot)) {
    throw "Set GDX_GODOT or pass -Godot with a Godot 4.x executable path."
}

if (Test-Path $Work) {
    Remove-Item -LiteralPath $Work -Recurse -Force
}

Invoke-Native cargo build -p gdx-cli

$Common = @("--godot", $Godot)

Invoke-Native $Bin project create --path $Work --name touchinput

[void](New-Item -ItemType Directory -Force -Path (Join-Path $Work "scripts"))
Set-Content -LiteralPath (Join-Path $Work "scripts\main.gd") -Encoding ASCII -Value @'
extends Node

var presses := 0
var releases := 0
var drags := 0
var last_log: Array = []

func _input(event: InputEvent) -> void:
    if event is InputEventScreenTouch:
        var touch := event as InputEventScreenTouch
        if touch.pressed:
            presses += 1
        else:
            releases += 1
        last_log.append({
            "kind": "touch",
            "index": touch.index,
            "pressed": touch.pressed,
            "position": [touch.position.x, touch.position.y],
        })
    elif event is InputEventScreenDrag:
        var drag := event as InputEventScreenDrag
        drags += 1
        last_log.append({
            "kind": "drag",
            "index": drag.index,
            "position": [drag.position.x, drag.position.y],
            "relative": [drag.relative.x, drag.relative.y],
        })

func gdx_state() -> Dictionary:
    return {
        "presses": presses,
        "releases": releases,
        "drags": drags,
        "last_log": last_log,
    }
'@

$Spec = Join-Path $Work "main_scene_spec.json"
Set-Content -LiteralPath $Spec -Encoding ASCII -Value @'
{
  "out": "res://scenes/main.tscn",
  "root": {
    "type": "Node",
    "name": "Main",
    "script": "res://scripts/main.gd",
    "children": []
  }
}
'@

Invoke-Native $Bin @Common --project $Work scene build --spec $Spec
Invoke-Native $Bin --project $Work setting set --section application --key run/main_scene --value "res://scenes/main.tscn"

try {
    $Start = Invoke-Json $Bin @Common --project $Work daemon start --width 400 --height 300 --restart
    if ($Start.ok -ne $true) { throw "daemon start did not return ok JSON" }
    if ($Start.capabilities.methods -notcontains "touch_sequence") { throw "daemon capabilities missing touch_sequence" }
    if ($Start.capabilities.input.touch -ne $true) { throw "daemon capabilities missing touch input flag" }
    if ($Start.capabilities.input.multi_touch -ne $true) { throw "daemon capabilities missing multi_touch input flag" }

    Invoke-Native $Bin --project $Work input tap --position 120 80 --frames 1
    $AfterTap = Invoke-Json $Bin --project $Work state get --target "/" --method gdx_state
    if ([int]$AfterTap.result.state.presses -ne 1) { throw "Expected one touch press after tap" }
    if ([int]$AfterTap.result.state.releases -ne 1) { throw "Expected one touch release after tap" }

    Invoke-Native $Bin --project $Work input drag --from 20 30 --to 80 90 --steps 2 --frames 1
    $AfterDrag = Invoke-Json $Bin --project $Work state get --target "/" --method gdx_state
    if ([int]$AfterDrag.result.state.presses -ne 2) { throw "Expected second touch press after drag" }
    if ([int]$AfterDrag.result.state.releases -ne 2) { throw "Expected second touch release after drag" }
    if ([int]$AfterDrag.result.state.drags -lt 2) { throw "Expected drag events after drag input" }

    $Pinch = Invoke-Json $Bin --project $Work input pinch --center 100 100 --start-distance 40 --end-distance 80 --steps 2 --frames 1
    if ($Pinch.result.after.active_touch_indexes.Count -ne 0) { throw "Expected pinch to release all touch indexes" }
    $AfterPinch = Invoke-Json $Bin --project $Work state get --target "/" --method gdx_state
    if ([int]$AfterPinch.result.state.presses -ne 4) { throw "Expected two additional touch presses after pinch" }
    if ([int]$AfterPinch.result.state.releases -ne 4) { throw "Expected two additional touch releases after pinch" }
    if ([int]$AfterPinch.result.state.drags -lt 6) { throw "Expected multi-touch drag events after pinch" }

    $SequenceSpec = Join-Path $Work "touch_sequence.json"
    Set-Content -LiteralPath $SequenceSpec -Encoding ASCII -Value @'
{
  "events": [
    { "kind": "touch", "index": 3, "position": [30, 40], "pressed": true },
    { "kind": "wait", "frames": 1 },
    { "kind": "drag", "index": 3, "position": [50, 70], "relative": [20, 30] },
    { "kind": "wait", "frames": 1 },
    { "kind": "touch", "index": 3, "position": [50, 70], "pressed": false }
  ]
}
'@
    Invoke-Native $Bin --project $Work input sequence --spec $SequenceSpec
    $AfterSequence = Invoke-Json $Bin --project $Work state get --target "/" --method gdx_state
    if ([int]$AfterSequence.result.state.presses -ne 5) { throw "Expected sequence touch press" }
    if ([int]$AfterSequence.result.state.releases -ne 5) { throw "Expected sequence touch release" }

    $VerifySpec = Join-Path $Work "verify.json"
    Set-Content -LiteralPath $VerifySpec -Encoding ASCII -Value @'
{
  "steps": [
    { "input_tap": { "position": [160, 120], "frames": 1 } },
    { "input_swipe": { "from": [160, 120], "to": [200, 160], "steps": 2, "frames": 1 } },
    { "input_pinch": { "center": [120, 120], "start_distance": 60, "end_distance": 30, "steps": 2, "frames": 1 } },
    { "input_touch_sequence": { "events": [
      { "kind": "touch", "index": 4, "position": [10, 10], "pressed": true },
      { "kind": "touch", "index": 4, "position": [10, 10], "pressed": false }
    ] } },
    { "state": { "target": "/", "method": "gdx_state" } }
  ]
}
'@
    $Verify = Invoke-Json $Bin --project $Work verify --spec $VerifySpec
    $VerifyState = $Verify.results[4].result.state
    if ([int]$VerifyState.presses -ne 10) { throw "Expected verify touch steps to add five presses" }
    if ([int]$VerifyState.releases -ne 10) { throw "Expected verify touch steps to add five releases" }
}
finally {
    & $Bin --project $Work daemon stop --force
}

Write-Host "GDX TOUCH INPUT E2E PASS: $Work"
