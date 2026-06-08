param(
    [string]$RepoRoot,
    [string]$Godot,
    [string]$WorkDir
)

$ErrorActionPreference = "Stop"

$scriptDir = Split-Path -Parent $PSCommandPath
$skillRoot = Resolve-Path -LiteralPath (Join-Path $scriptDir "..")
$gdx = & (Join-Path $scriptDir "resolve-gdx.ps1") -RepoRoot $RepoRoot

if (-not $WorkDir) {
    $WorkDir = Join-Path ([System.IO.Path]::GetTempPath()) ("gdx-skill-smoke-" + [System.Guid]::NewGuid().ToString("N"))
}

$project = Join-Path $WorkDir "demo"
$capture = Join-Path $project ".gdx\capture.png"
$testDir = Join-Path $project "tests"
$specPath = Join-Path $WorkDir "hello-2d.json"
$testPath = Join-Path $testDir "smoke_test.gd"

$globalArgs = @()
if ($Godot) {
    $globalArgs += @("--godot", $Godot)
}

New-Item -ItemType Directory -Path $WorkDir -Force | Out-Null
Copy-Item -LiteralPath (Join-Path $skillRoot "assets\scene-specs\hello-2d.json") -Destination $specPath -Force

& $gdx @globalArgs doctor
& $gdx @globalArgs project create --path $project --name GdxSkillSmoke
New-Item -ItemType Directory -Path $testDir -Force | Out-Null
Copy-Item -LiteralPath (Join-Path $skillRoot "assets\scripts\smoke_test.gd") -Destination $testPath -Force

& $gdx @globalArgs --project $project scene build --spec $specPath
& $gdx @globalArgs --project $project asset import
& $gdx @globalArgs --project $project script check-all
& $gdx @globalArgs --project $project test run --path res://tests/smoke_test.gd
& $gdx @globalArgs --project $project capture run --scene res://scenes/main.tscn --out $capture

if (-not (Test-Path -LiteralPath $capture -PathType Leaf)) {
    throw "Capture was not created: $capture"
}

if ((Get-Item -LiteralPath $capture).Length -le 0) {
    throw "Capture is empty: $capture"
}

Write-Output (@{
    ok = $true
    project = $project
    capture = $capture
} | ConvertTo-Json -Compress)
