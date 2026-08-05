param(
    [string]$Godot = $env:GDX_GODOT,
    [switch]$SkipBuild
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$Root = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
$Bin = Join-Path $Root "target\debug\gdx.exe"
$Work = Join-Path $env:TEMP ("gdx_daemon_runtime_contract_" + [guid]::NewGuid().ToString("N"))
$Project = Join-Path $Work "project"
$Stopped = $false

function Assert-Contract {
    param(
        [Parameter(Mandatory = $true)]
        [bool]$Condition,

        [Parameter(Mandatory = $true)]
        [string]$Message
    )

    if (-not $Condition) {
        throw $Message
    }
}

function Invoke-GdxRedirected {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Name,

        [Parameter(Mandatory = $true)]
        [string[]]$Arguments
    )

    $stdoutPath = Join-Path $Work "$Name.stdout.json"
    $stderrPath = Join-Path $Work "$Name.stderr.ndjson"
    $quotedArguments = @(
        foreach ($argument in $Arguments) {
            if ($argument -match '[\s"]') {
                '"' + ($argument -replace '"', '\"') + '"'
            } else {
                $argument
            }
        }
    )
    $process = Start-Process -FilePath $Bin `
        -ArgumentList ($quotedArguments -join " ") `
        -RedirectStandardOutput $stdoutPath `
        -RedirectStandardError $stderrPath `
        -NoNewWindow `
        -PassThru
    $processHandle = $process.Handle
    [void]$process.WaitForExit()
    $process.Refresh()
    $stdout = if (Test-Path -LiteralPath $stdoutPath) {
        Get-Content -Raw -LiteralPath $stdoutPath
    } else {
        ""
    }
    $stderr = if (Test-Path -LiteralPath $stderrPath) {
        Get-Content -Raw -LiteralPath $stderrPath
    } else {
        ""
    }
    $events = @(
        $stderr -split "\r?\n" |
            Where-Object { -not [string]::IsNullOrWhiteSpace($_) } |
            ForEach-Object { $_ | ConvertFrom-Json }
    )
    $result = if ([string]::IsNullOrWhiteSpace($stdout)) {
        $null
    } else {
        $stdout | ConvertFrom-Json
    }
    [pscustomobject]@{
        ExitCode = $process.ExitCode
        Result = $result
        Events = $events
        Stdout = $stdout
        Stderr = $stderr
    }
}

function Invoke-GdxSuccess {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Name,

        [Parameter(Mandatory = $true)]
        [string[]]$Arguments
    )

    $invocation = Invoke-GdxRedirected -Name $Name -Arguments $Arguments
    Assert-Contract ($invocation.ExitCode -eq 0) "$Name failed: $($invocation.Stderr)"
    Assert-Contract ($null -ne $invocation.Result) "$Name did not emit stdout JSON"
    return $invocation.Result
}

if ([string]::IsNullOrWhiteSpace($Godot)) {
    throw "Set GDX_GODOT or pass -Godot with a Godot 4.x executable path."
}

if (-not $SkipBuild) {
    & cargo build -p gdx-cli
    if ($LASTEXITCODE -ne 0) {
        throw "cargo build -p gdx-cli failed with exit code $LASTEXITCODE"
    }
}

Assert-Contract (Test-Path -LiteralPath $Bin) "gdx binary not found: $Bin"
[System.IO.Directory]::CreateDirectory($Work) | Out-Null

$Common = @("--godot", $Godot, "--project", $Project, "--progress", "ndjson")
[void](Invoke-GdxSuccess -Name "project-create" -Arguments @(
    "--progress", "ndjson", "project", "create", "--path", $Project, "--name", "Daemon Contract"
))
[void](Invoke-GdxSuccess -Name "project-update" -Arguments @(
    "--project", $Project, "--progress", "ndjson", "project", "update"
))
[void](Invoke-GdxSuccess -Name "set-renderer" -Arguments @(
    "--project", $Project, "--progress", "ndjson", "setting", "set",
    "--section", "rendering", "--key", "renderer/rendering_method", "--value", "gl_compatibility"
))

[System.IO.Directory]::CreateDirectory((Join-Path $Project "scripts")) | Out-Null
[System.IO.Directory]::CreateDirectory((Join-Path $Project "design")) | Out-Null
[System.IO.File]::WriteAllText((Join-Path $Project "scripts\main.gd"), @'
extends Node

func gdx_finite() -> Dictionary:
    return { "version": 1, "nested": { "value": 12.5 } }

func gdx_non_finite() -> Dictionary:
    return {
        "positive": INF,
        "negative": -INF,
        "nested": [{ "nan": NAN }, Vector2(INF, 2.0)],
        "escaped/key~": INF,
    }
'@, [System.Text.Encoding]::ASCII)
[System.IO.File]::WriteAllText(
    (Join-Path $Project "scripts\removed.gd"),
    "extends Node`n",
    [System.Text.Encoding]::ASCII
)
[System.IO.File]::WriteAllText(
    (Join-Path $Project "design\.gdignore"),
    "",
    [System.Text.Encoding]::ASCII
)
[System.IO.File]::WriteAllText(
    (Join-Path $Project "design\reference.gd"),
    "extends Node`n",
    [System.Text.Encoding]::ASCII
)

$Spec = Join-Path $Work "main_scene_spec.json"
[System.IO.File]::WriteAllText($Spec, @'
{
  "out": "res://scenes/main.tscn",
  "root": {
    "type": "Node",
    "name": "Main",
    "script": "res://scripts/main.gd",
    "children": []
  }
}
'@, [System.Text.Encoding]::ASCII)
[void](Invoke-GdxSuccess -Name "scene-build" -Arguments ($Common + @(
    "scene", "build", "--spec", $Spec
)))
[void](Invoke-GdxSuccess -Name "set-main" -Arguments @(
    "--project", $Project, "--progress", "ndjson", "setting", "set",
    "--section", "application", "--key", "run/main_scene", "--value", "res://scenes/main.tscn"
))

try {
    $start = Invoke-GdxSuccess -Name "daemon-start" -Arguments ($Common + @(
        "daemon", "start", "--width", "320", "--height", "240", "--restart"
    ))
    Assert-Contract ($start.health.rpc_ready -eq $true) "daemon RPC did not become ready"
    Assert-Contract ($start.health.scene_loaded -eq $true) "daemon scene did not load"
    Assert-Contract ($start.health.project_healthy -eq $true) "daemon project is unhealthy"
    Assert-Contract ($start.runtime.version -eq "0.1.2") "daemon runtime version is not 0.1.2"

    $finite = Invoke-GdxSuccess -Name "finite" -Arguments ($Common + @(
        "call", "invoke", "--target", "/", "--method", "gdx_finite"
    ))
    Assert-Contract ([int]$finite.result.result.version -eq 1) "finite RPC returned the wrong version"

    $nonFinite = Invoke-GdxRedirected -Name "non-finite" -Arguments ($Common + @(
        "call", "invoke", "--target", "/", "--method", "gdx_non_finite"
    ))
    Assert-Contract ($nonFinite.ExitCode -ne 0) "non-finite RPC unexpectedly succeeded"
    Assert-Contract ($null -eq $nonFinite.Result) "non-finite RPC emitted stdout JSON"
    $nonFiniteFailure = $nonFinite.Events[-1]
    Assert-Contract ($nonFiniteFailure.error -eq "daemon_non_finite_response") "wrong non-finite error code"
    Assert-Contract ([int]$nonFiniteFailure.details.count -eq 5) "wrong non-finite value count"
    $paths = @($nonFiniteFailure.details.non_finite_values | ForEach-Object { $_.path })
    foreach ($expectedPath in @(
        "/escaped~1key~0",
        "/negative",
        "/nested/0/nan",
        "/nested/1/vec2/0",
        "/positive"
    )) {
        Assert-Contract ($paths -contains $expectedPath) "missing non-finite JSON pointer: $expectedPath"
    }

    $freshStatus = Invoke-GdxSuccess -Name "status-fresh" -Arguments ($Common + @(
        "daemon", "status"
    ))
    Assert-Contract ($freshStatus.script_sync.status -eq "fresh") "fresh daemon reported stale scripts"

    [System.IO.File]::WriteAllText((Join-Path $Project "scripts\main.gd"), @'
extends Node

func gdx_finite() -> Dictionary:
    return { "version": 2, "nested": { "value": 12.5 } }
'@, [System.Text.Encoding]::ASCII)
    [System.IO.File]::WriteAllText(
        (Join-Path $Project "scripts\added.gd"),
        "extends Node`n",
        [System.Text.Encoding]::ASCII
    )
    [System.IO.File]::Delete((Join-Path $Project "scripts\removed.gd"))
    [System.IO.File]::WriteAllText(
        (Join-Path $Project "design\reference.gd"),
        "extends Node`nfunc ignored():`n    pass`n",
        [System.Text.Encoding]::ASCII
    )

    $staleCall = Invoke-GdxRedirected -Name "stale-call" -Arguments ($Common + @(
        "call", "invoke", "--target", "/", "--method", "gdx_finite"
    ))
    Assert-Contract ($staleCall.ExitCode -ne 0) "stale daemon call unexpectedly succeeded"
    $staleFailure = $staleCall.Events[-1]
    Assert-Contract ($staleFailure.error -eq "daemon_scripts_stale") "wrong stale script error code"
    Assert-Contract ([int]$staleFailure.details.total_changes -eq 3) "ignored or missing script change"
    Assert-Contract ($staleFailure.details.modified -contains "res://scripts/main.gd") "modified script not reported"
    Assert-Contract ($staleFailure.details.added -contains "res://scripts/added.gd") "added script not reported"
    Assert-Contract ($staleFailure.details.removed -contains "res://scripts/removed.gd") "removed script not reported"

    $staleStatus = Invoke-GdxSuccess -Name "status-stale" -Arguments ($Common + @(
        "daemon", "status"
    ))
    Assert-Contract ($staleStatus.running -eq $true) "stale daemon should remain running"
    Assert-Contract ($staleStatus.script_sync.status -eq "stale") "status did not report stale scripts"

    $reuse = Invoke-GdxRedirected -Name "start-without-restart" -Arguments ($Common + @(
        "daemon", "start"
    ))
    Assert-Contract ($reuse.ExitCode -ne 0) "daemon start reused stale scripts"
    Assert-Contract ($reuse.Events[-1].error -eq "daemon_scripts_stale") "stale reuse returned wrong error"

    $restart = Invoke-GdxSuccess -Name "daemon-restart" -Arguments ($Common + @(
        "daemon", "start", "--width", "320", "--height", "240", "--restart"
    ))
    Assert-Contract ($restart.already_running -eq $false) "restart did not create a fresh daemon"
    $updated = Invoke-GdxSuccess -Name "finite-updated" -Arguments ($Common + @(
        "call", "invoke", "--target", "/", "--method", "gdx_finite"
    ))
    Assert-Contract ([int]$updated.result.result.version -eq 2) "restart did not load the updated script"

    [System.IO.File]::AppendAllText(
        (Join-Path $Project "scripts\main.gd"),
        "`nfunc changed_again() -> void:`n    pass`n",
        [System.Text.Encoding]::ASCII
    )
    $stop = Invoke-GdxSuccess -Name "stop-stale" -Arguments ($Common + @(
        "daemon", "stop", "--force"
    ))
    Assert-Contract ($stop.killed -eq $true) "daemon stop did not work after scripts became stale"
    $Stopped = $true
}
finally {
    if (-not $Stopped -and (Test-Path -LiteralPath $Bin)) {
        & $Bin --project $Project --progress off daemon stop --force
    }
}

Write-Host "GDX DAEMON RUNTIME CONTRACT E2E PASS: $Work"
