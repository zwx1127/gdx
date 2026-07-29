param(
    [switch]$SkipBuild
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$Root = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
$Bin = Join-Path $Root "target\debug\gdx.exe"
$Work = Join-Path $env:TEMP ("gdx_cli_contract_" + [guid]::NewGuid().ToString("N"))
$Project = Join-Path $Work "project with spaces"

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
        [string[]]$Arguments,

        [AllowNull()]
        [string]$StandardInput
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
    $startParameters = @{
        FilePath = $Bin
        ArgumentList = ($quotedArguments -join " ")
        RedirectStandardOutput = $stdoutPath
        RedirectStandardError = $stderrPath
        NoNewWindow = $true
        PassThru = $true
        Wait = $true
    }
    if ($PSBoundParameters.ContainsKey("StandardInput")) {
        $stdinPath = Join-Path $Work "$Name.stdin.json"
        [System.IO.File]::WriteAllText($stdinPath, $StandardInput, [System.Text.UTF8Encoding]::new($false))
        $startParameters.RedirectStandardInput = $stdinPath
    }
    $process = Start-Process @startParameters
    $exitCode = $process.ExitCode
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
    $stderrValues = @(
        $stderr -split "\r?\n" |
            Where-Object { -not [string]::IsNullOrWhiteSpace($_) } |
            ForEach-Object { $_ | ConvertFrom-Json }
    )
    $stdoutValue = if ([string]::IsNullOrWhiteSpace($stdout)) {
        $null
    } else {
        $stdout | ConvertFrom-Json
    }
    [pscustomobject]@{
        ExitCode = $exitCode
        Stdout = $stdout
        Stderr = $stderr
        Result = $stdoutValue
        Events = $stderrValues
    }
}

if (-not $SkipBuild) {
    & cargo build -p gdx-cli
    if ($LASTEXITCODE -ne 0) {
        throw "cargo build -p gdx-cli failed with exit code $LASTEXITCODE"
    }
}

if (-not (Test-Path -LiteralPath $Bin)) {
    throw "gdx binary not found: $Bin"
}

[System.IO.Directory]::CreateDirectory($Work) | Out-Null

$create = Invoke-GdxRedirected -Name "create" -Arguments @(
    "--progress", "ndjson",
    "project", "create",
    "--path", $Project,
    "--name", "Contract Game"
)
Assert-Contract ($create.ExitCode -eq 0) "Project create failed: $($create.Stderr)"
Assert-Contract ($create.Result.command -eq "project.create") "Unexpected project create result"
Assert-Contract ($create.Result.PSObject.Properties.Name -notcontains "gdx") "Build provenance leaked outside doctor"
Assert-Contract (($create.Stdout.Trim() -split "\r?\n").Count -eq 1) "Success output must be one compact JSON line"
Assert-Contract (@($create.Events | Where-Object { $_.event -eq "command_started" }).Count -eq 1) "Missing command_started event"
Assert-Contract (@($create.Events | Where-Object { $_.event -eq "completed" -and $_.ok }).Count -eq 1) "Missing completed event"
Assert-Contract (Test-Path -LiteralPath (Join-Path $Project "project.godot")) "Project was not created"

$gitignore = [System.IO.File]::ReadAllText((Join-Path $Project ".gitignore"))
Assert-Contract ($gitignore.Trim() -eq ".godot/") "Scaffold .gitignore must contain only .godot/"

foreach ($index in 1..3) {
    $inspect = Invoke-GdxRedirected -Name "inspect-$index" -Arguments @(
        "--project", $Project,
        "--progress", "off",
        "project", "inspect"
    )
    Assert-Contract ($inspect.ExitCode -eq 0) "Project inspect failed: $($inspect.Stderr)"
    Assert-Contract ($inspect.Result.command -eq "project.inspect") "Unexpected inspect result"
    Assert-Contract ($inspect.Result.PSObject.Properties.Name -notcontains "files") "Inspect returned an unbounded file listing"
}
Assert-Contract (-not (Test-Path -LiteralPath (Join-Path $Project ".gdx\runs"))) "Repeated commands created persisted run records"
Assert-Contract (-not (Test-Path -LiteralPath (Join-Path $Project ".gdx\daemon"))) "Repeated commands created project-local daemon state"

$invalid = Invoke-GdxRedirected -Name "invalid-stdin" -StandardInput '{"broken":' -Arguments @(
    "--project", $Project,
    "--progress", "ndjson",
    "call", "invoke",
    "--target", "/",
    "--method", "gdx_contract",
    "--args-json", "-"
)
Assert-Contract ($invalid.ExitCode -ne 0) "Invalid stdin JSON unexpectedly succeeded"
Assert-Contract ($null -eq $invalid.Result) "Failed command emitted stdout JSON"
$failure = $invalid.Events[-1]
Assert-Contract ($failure.ok -eq $false) "Final stderr line is not the structured failure"
Assert-Contract ($failure.error -eq "invalid_args_json") "Invalid stdin JSON returned the wrong error"
Assert-Contract ($failure.details.input_origin -eq "stdin") "Invalid stdin JSON did not report stdin origin"
Assert-Contract ($failure.details.PSObject.Properties.Name -notcontains "preview") "Error copied an input preview"

$stringCall = Invoke-GdxRedirected -Name "string-args" -Arguments @(
    "--project", $Project,
    "--progress", "off",
    "call", "invoke",
    "--target", "/",
    "--method", "gdx_contract",
    "--arg", "ASCII",
    "--arg", "argument with spaces"
)
Assert-Contract ($stringCall.ExitCode -ne 0) "Call without a daemon unexpectedly succeeded"
Assert-Contract ($stringCall.Events[-1].error -eq "daemon_not_running") "Repeatable string arguments were rejected"

$removedShapes = @(
    @("--progress-file", "progress.ndjson", "project", "inspect"),
    @("--project", $Project, "runs", "list"),
    @("--project", $Project, "project", "install"),
    @("--project", $Project, "project", "inspect", "--max-files", "1"),
    @("project", "create", "--path", (Join-Path $Work "forced"), "--name", "Forced", "--force"),
    @("--project", $Project, "asset", "copy", "--from", "a", "--to", "res://a"),
    @("--project", $Project, "input", "swipe", "--from", "0", "0", "--to", "1", "1"),
    @("--project", $Project, "input", "send", "--mouse-button", "1", "--release"),
    @("--project", $Project, "call", "invoke", "--target", "/", "--method", "x", "--args-file", "args.json"),
    @("--project", $Project, "call", "invoke", "--target", "/", "--method", "x", "--args-stdin"),
    @("--project", $Project, "--warnings", "summary", "project", "inspect")
)
for ($index = 0; $index -lt $removedShapes.Count; $index++) {
    $removed = Invoke-GdxRedirected -Name "removed-$index" -Arguments $removedShapes[$index]
    Assert-Contract ($removed.ExitCode -eq 2) "Removed CLI shape was still accepted: $($removedShapes[$index] -join ' ')"
    Assert-Contract ($removed.Events[-1].error -eq "invalid_cli_arguments") "Removed CLI shape did not return a structured clap error"
}

Write-Host "GDX CLI CONTRACT E2E PASS: $Work"
