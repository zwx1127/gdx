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

function Assert-ImageHasVisibleContent {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Path
    )

    Add-Type -AssemblyName System.Drawing
    $Bitmap = [System.Drawing.Bitmap]::new($Path)
    try {
        $Background = $Bitmap.GetPixel(0, 0)
        $DifferentPixels = 0
        $StepX = 4
        $StepY = 4
        for ($Y = 0; $Y -lt $Bitmap.Height; $Y += $StepY) {
            for ($X = 0; $X -lt $Bitmap.Width; $X += $StepX) {
                $Pixel = $Bitmap.GetPixel($X, $Y)
                $Delta = [Math]::Abs($Pixel.R - $Background.R) + [Math]::Abs($Pixel.G - $Background.G) + [Math]::Abs($Pixel.B - $Background.B)
                if ($Delta -gt 24) {
                    $DifferentPixels += 1
                    if ($DifferentPixels -ge 50) {
                        return
                    }
                }
            }
        }
        throw "Capture does not contain visible rendered 3D content: $Path"
    }
    finally {
        $Bitmap.Dispose()
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

Invoke-Native $Bin project create --path $Work --name daemon3d
Invoke-Native $Bin @Common --project $Work scene create `
    --out "res://scenes/main_3d.tscn" `
    --root-type Node3D `
    --name Main3D `
    --set-main
Invoke-Native $Bin @Common --project $Work asset import

[void](New-Item -ItemType Directory -Force -Path (Join-Path $Work "meshes"))
[void](New-Item -ItemType Directory -Force -Path (Join-Path $Work "materials"))
$MaterialProps = Join-Path $Work "box_material.json"
Set-Content -LiteralPath $MaterialProps -Encoding ASCII -Value '{ "albedo_color": { "color": [1.0, 0.45, 0.1, 1.0] } }'
Invoke-Native $Bin @Common --project $Work resource create --type BoxMesh --out "res://meshes/box.tres"
Invoke-Native $Bin @Common --project $Work resource create --type StandardMaterial3D --out "res://materials/box_material.tres" --properties $MaterialProps

try {
    Invoke-Native $Bin @Common --project $Work daemon start `
        --width 1280 `
        --height 720 `
        --restart

    Invoke-Native $Bin --project $Work scene tree
    Invoke-Native $Bin --project $Work node create --parent "/" --type Camera3D --name Camera
    Invoke-Native $Bin --project $Work node set --node "/Camera" --property position --vec3 0 3 6
    Invoke-Native $Bin --project $Work node set --node "/Camera" --property rotation_degrees --vec3 -25 0 0
    Invoke-Native $Bin --project $Work node set --node "/Camera" --property current --bool true
    Invoke-Native $Bin --project $Work node create --parent "/" --type DirectionalLight3D --name Sun
    Invoke-Native $Bin --project $Work node set --node "/Sun" --property rotation_degrees --vec3 -45 -30 0
    Invoke-Native $Bin --project $Work node create --parent "/" --type MeshInstance3D --name AddedBox

    Invoke-Native $Bin --project $Work node set --node "/AddedBox" --property mesh --resource "res://meshes/box.tres"
    Invoke-Native $Bin --project $Work node set --node "/AddedBox" --property material_override --resource "res://materials/box_material.tres"
    Invoke-Native $Bin --project $Work node set --node "/AddedBox" --property position --vec3 0 0.5 0
    Invoke-Native $Bin --project $Work scene save

    $SceneText = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $Work "scenes\main_3d.tscn")
    if ($SceneText -notmatch "AddedBox") { throw "Saved scene does not include AddedBox" }

    Invoke-Native $Bin --project $Work capture daemon --out $Shot --frames 10

    $ShotInfo = Get-Item -LiteralPath $Shot
    if ($ShotInfo.Length -le 0) { throw "Capture is empty: $Shot" }
    Assert-ImageHasVisibleContent -Path $Shot
}
finally {
    & $Bin --project $Work daemon stop --force
}

Write-Host "GDX DAEMON 3D E2E PASS: $Shot"
