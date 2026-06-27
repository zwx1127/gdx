param(
    [string]$Godot = $env:GDX_GODOT
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$Root = Resolve-Path (Join-Path $PSScriptRoot "..\..")
$Bin = Join-Path $Root "target\debug\gdx.exe"
$Work = Join-Path $env:TEMP ("gdx_ui_click_" + [guid]::NewGuid().ToString("N"))

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

Invoke-Native $Bin project create --path $Work --name uiclick

[void](New-Item -ItemType Directory -Force -Path (Join-Path $Work "scripts"))
Set-Content -LiteralPath (Join-Path $Work "scripts\main.gd") -Encoding ASCII -Value @'
extends Control

var clicks := 0

func _ready() -> void:
    var button := Button.new()
    button.name = "ClickMe"
    button.text = "Click"
    button.position = Vector2(40, 40)
    button.size = Vector2(180, 80)
    button.pressed.connect(_on_button_pressed)
    add_child(button)

func _on_button_pressed() -> void:
    clicks += 1

func gdx_state() -> Dictionary:
    return { "clicks": clicks }
'@

$Spec = Join-Path $Work "main_scene_spec.json"
Set-Content -LiteralPath $Spec -Encoding ASCII -Value @'
{
  "out": "res://scenes/main.tscn",
  "root": {
    "type": "Control",
    "name": "Main",
    "script": "res://scripts/main.gd",
    "properties": {
      "layout_mode": 3,
      "anchors_preset": 15
    },
    "children": []
  }
}
'@

Invoke-Native $Bin @Common --project $Work scene build --spec $Spec
Invoke-Native $Bin --project $Work setting set --section application --key run/main_scene --value "res://scenes/main.tscn"

$SceneText = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $Work "scenes\main.tscn")
if ($SceneText -notmatch "res://scripts/main.gd") { throw "Saved scene does not reference main.gd" }
if ($SceneText -notmatch "script = ExtResource") { throw "Saved scene does not attach root script" }

try {
    $Start = Invoke-Json $Bin @Common --project $Work daemon start --width 400 --height 300 --restart
    if ($Start.ok -ne $true) { throw "daemon start did not return ok JSON" }
    if ($Start.capabilities.status -ne "known") { throw "daemon start did not report known capabilities" }
    if ($Start.runtime_status -ne "known") { throw "daemon start did not promote known runtime status" }
    if ([int]$Start.pid -le 0) { throw "daemon start did not promote pid" }
    if ([int]$Start.port -le 0) { throw "daemon start did not promote port" }
    if ($Start.methods -notcontains "click_node") { throw "daemon start did not promote methods" }
    if ($Start.capabilities.methods -notcontains "click_node") { throw "daemon capabilities missing click_node" }
    if ($Start.capabilities.methods -notcontains "activate_node") { throw "daemon capabilities missing activate_node" }

    $Status = Invoke-Json $Bin --project $Work daemon status
    if ($Status.running -ne $true) { throw "Expected daemon status to be running" }
    if ($Status.capabilities.status -ne "known") { throw "daemon status did not report known capabilities" }
    if ($Status.runtime_status -ne "known") { throw "daemon status did not promote known runtime status" }
    if ($Status.scene -ne "res://scenes/main.tscn") { throw "daemon status did not promote scene" }

    $Tree = Invoke-Json $Bin --project $Work scene tree --include-script --include-groups --include-methods
    if ($Tree.tree.methods -notcontains "gdx_state") { throw "scene tree diagnostics missing gdx_state method" }

    $Before = Invoke-Json $Bin --project $Work state get --target "/"
    if ($Before.result.source -ne "method") { throw "Expected state get to use method source" }
    if ($Before.result.method -ne "gdx_state") { throw "Expected state get to default to gdx_state" }
    if ([int]$Before.result.state.clicks -ne 0) { throw "Expected zero clicks before input" }

    Invoke-Native $Bin --project $Work input click --position 120 80 --frames 2

    $After = Invoke-Json $Bin --project $Work state get --target "/" --method gdx_state
    if ([int]$After.result.state.clicks -ne 1) { throw "Expected one click after input" }

    Invoke-Native $Bin --project $Work input click-node --target "/ClickMe" --frames 2

    $AfterNode = Invoke-Json $Bin --project $Work state get --target "/" --method gdx_state
    if ([int]$AfterNode.result.state.clicks -ne 2) { throw "Expected two clicks after node input" }

    Invoke-Native $Bin --project $Work input activate --target "/ClickMe"

    $AfterActivate = Invoke-Json $Bin --project $Work state get --target "/" --method gdx_state
    if ([int]$AfterActivate.result.state.clicks -ne 3) { throw "Expected three clicks after activate input" }

    $VerifySpec = Join-Path $Work "verify.json"
    Set-Content -LiteralPath $VerifySpec -Encoding ASCII -Value @'
{
  "steps": [
    { "state": { "target": "/" } },
    { "input_activate": { "target": "/ClickMe" } },
    { "state": { "target": "/", "method": "gdx_state" } }
  ]
}
'@
    $Verify = Invoke-Json $Bin --project $Work verify --spec $VerifySpec
    if ($Verify.results[0].result.method -ne "gdx_state") { throw "Expected verify state step to default to gdx_state" }
    if ([int]$Verify.results[2].result.state.clicks -ne 4) { throw "Expected verify input step to activate button" }
}
finally {
    & $Bin --project $Work daemon stop --force
}

Write-Host "GDX UI CLICK E2E PASS: $Work"
