param(
    [string]$RepoRoot
)

$ErrorActionPreference = "Stop"

function Resolve-RepoRoot {
    param([string]$ExplicitRoot)

    if ($ExplicitRoot) {
        return (Resolve-Path -LiteralPath $ExplicitRoot).Path
    }

    $scriptPath = $PSCommandPath
    if (-not $scriptPath) {
        return (Get-Location).Path
    }

    return (Resolve-Path -LiteralPath (Join-Path (Split-Path -Parent $scriptPath) "..\..\..")).Path
}

$pathCommand = Get-Command gdx -ErrorAction SilentlyContinue
if ($pathCommand) {
    $pathCommand.Source
    exit 0
}

$root = Resolve-RepoRoot -ExplicitRoot $RepoRoot
$candidates = @(
    (Join-Path $root "target\debug\gdx.exe"),
    (Join-Path $root "target\debug\gdx")
)

foreach ($candidate in $candidates) {
    if (Test-Path -LiteralPath $candidate -PathType Leaf) {
        (Resolve-Path -LiteralPath $candidate).Path
        exit 0
    }
}

throw "Could not find gdx on PATH or under '$root\target\debug'. Build it with: cargo build --workspace"
