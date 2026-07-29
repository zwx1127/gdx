param(
    [string]$Godot = $env:GDX_GODOT
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$Root = Resolve-Path (Join-Path $PSScriptRoot "..\..")
$Bin = Join-Path $Root "target\debug\gdx.exe"
$Work = Join-Path $env:TEMP ("gdx_touch_outdated_" + [guid]::NewGuid().ToString("N"))

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

function Invoke-FailingJson {
    param(
        [Parameter(Mandatory = $true)]
        [string]$FilePath,

        [Parameter(ValueFromRemainingArguments = $true)]
        [string[]]$Arguments
    )

    $StdoutPath = Join-Path $Work "failed-command.stdout.json"
    $StderrPath = Join-Path $Work "failed-command.stderr.json"
    $Process = Start-Process -FilePath $FilePath `
        -ArgumentList $Arguments `
        -RedirectStandardOutput $StdoutPath `
        -RedirectStandardError $StderrPath `
        -Wait `
        -PassThru `
        -NoNewWindow
    $ExitCode = $Process.ExitCode
    $Stdout = if (Test-Path $StdoutPath) { Get-Content -Raw -LiteralPath $StdoutPath } else { "" }
    $Stderr = if (Test-Path $StderrPath) { Get-Content -Raw -LiteralPath $StderrPath } else { "" }
    if ($ExitCode -eq 0) {
        throw "Expected command to fail but it succeeded: $FilePath $($Arguments -join ' ')`n$Stdout"
    }
    if ([string]::IsNullOrWhiteSpace($Stderr)) {
        throw "Failed command did not emit JSON on stderr: $FilePath $($Arguments -join ' ')`nstdout:`n$Stdout"
    }
    $FinalLine = ($Stderr -split "`r?`n" | Where-Object { -not [string]::IsNullOrWhiteSpace($_) } | Select-Object -Last 1)
    return $FinalLine | ConvertFrom-Json
}

if ([string]::IsNullOrWhiteSpace($Godot)) {
    throw "Set GDX_GODOT or pass -Godot with a Godot 4.x executable path."
}

if (Test-Path $Work) {
    Remove-Item -LiteralPath $Work -Recurse -Force
}

Invoke-Native cargo build -p gdx-cli

$Common = @("--godot", $Godot)

Invoke-Native $Bin project create --path $Work --name touchoutdated
Invoke-Native $Bin @Common --project $Work scene create `
    --out "res://scenes/main.tscn" `
    --root-type Node2D `
    --name Main `
    --set-main

$DaemonServer = Join-Path $Work "addons\gdx_daemon\daemon_server.gd"
$ServerText = Get-Content -Raw -Encoding UTF8 -LiteralPath $DaemonServer
if ($ServerText -notmatch '"touch_sequence"') {
    throw "Fresh project daemon runtime did not advertise touch_sequence before fixture edit"
}
$ServerText = [regex]::Replace($ServerText, '(?m)^\s+"touch_sequence",\r?\n', '', 1)
if ($ServerText -match '(?m)^\s+"touch_sequence",\s*$') {
    throw "Failed to remove touch_sequence from daemon capabilities fixture"
}
[System.IO.File]::WriteAllText($DaemonServer, $ServerText, [System.Text.UTF8Encoding]::new($false))

try {
    $ErrorJson = Invoke-FailingJson $Bin @Common --project $Work daemon start --width 320 --height 240 --restart
    if ($ErrorJson.error -ne "daemon_runtime_outdated") { throw "Expected daemon_runtime_outdated, got $($ErrorJson.error)" }
    if ($ErrorJson.stage -ne "runtime_preflight") { throw "Expected runtime_preflight stage, got $($ErrorJson.stage)" }
    if ($ErrorJson.details.managed_runtime.status -ne "outdated") {
        throw "Expected managed runtime status outdated, got $($ErrorJson.details.managed_runtime.status)"
    }
    if ($ErrorJson.details.managed_runtime.compatible -ne $false) {
        throw "Outdated managed runtime unexpectedly reported compatible"
    }
    if ($ErrorJson.suggestion -notmatch "project update") { throw "Error suggestion did not mention project update" }

    $Status = Invoke-Json $Bin --project $Work daemon status
    if ($Status.running -ne $false) { throw "Preflight failure unexpectedly left a daemon running" }
    if ($Status.runtime.status -ne "not_running") { throw "daemon status did not report not_running" }

    $Check = Invoke-Json $Bin --project $Work project update --check
    if ($Check.up_to_date -ne $false) { throw "project update --check did not detect the modified runtime" }
    if (@($Check.changes | Where-Object { $_.status_before -eq "outdated" }).Count -lt 1) {
        throw "project update --check did not report an outdated managed file"
    }

    $Update = Invoke-Json $Bin --project $Work project update
    if ($Update.up_to_date -ne $true) { throw "project update did not restore the managed runtime" }

    $Start = Invoke-Json $Bin @Common --project $Work daemon start --width 320 --height 240 --restart
    if ($Start.ok -ne $true) { throw "daemon start did not return ok JSON after update" }
    if ($Start.runtime.status -ne "current") { throw "Expected daemon runtime status current, got $($Start.runtime.status)" }
    if ($Start.health.project_healthy -ne $true) { throw "Updated daemon did not pass project health" }

    $Tap = Invoke-Json $Bin --project $Work input tap --position 10 10
    if ($Tap.ok -ne $true) { throw "Touch input failed after restoring the managed runtime" }
}
finally {
    & $Bin --project $Work daemon stop --force
}

Write-Host "GDX TOUCH RUNTIME OUTDATED E2E PASS: $Work"
