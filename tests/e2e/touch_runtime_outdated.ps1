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
    return $Stderr | ConvertFrom-Json
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
    $Start = Invoke-Json $Bin @Common --project $Work daemon start --width 320 --height 240 --restart
    if ($Start.ok -ne $true) { throw "daemon start did not return ok JSON" }
    if ($Start.runtime_status -ne "outdated") { throw "Expected daemon start runtime_status outdated, got $($Start.runtime_status)" }
    if ($Start.methods -contains "touch_sequence") { throw "Outdated fixture unexpectedly advertised touch_sequence" }
    if (-not (($Start.warnings -join "`n") -match "project update --check")) {
        throw "daemon start warning did not mention project update --check"
    }

    $Status = Invoke-Json $Bin --project $Work daemon status
    if ($Status.running -ne $true) { throw "Expected daemon status to be running" }
    if ($Status.runtime_status -ne "outdated") { throw "Expected daemon status runtime_status outdated, got $($Status.runtime_status)" }
    if (-not (($Status.warnings -join "`n") -match "project update --check")) {
        throw "daemon status warning did not mention project update --check"
    }

    $ErrorJson = Invoke-FailingJson $Bin --project $Work input tap --position 10 10
    if ($ErrorJson.error -ne "daemon_runtime_outdated") { throw "Expected daemon_runtime_outdated, got $($ErrorJson.error)" }
    if ($ErrorJson.details.requested_rpc_method -ne "touch_sequence") { throw "Expected requested_rpc_method touch_sequence" }
    if ($ErrorJson.details.capabilities.methods -contains "touch_sequence") { throw "Error capabilities unexpectedly advertised touch_sequence" }
    if ($ErrorJson.suggestion -notmatch "project update --check") { throw "Error suggestion did not mention project update --check" }
}
finally {
    & $Bin --project $Work daemon stop --force
}

Write-Host "GDX TOUCH RUNTIME OUTDATED E2E PASS: $Work"