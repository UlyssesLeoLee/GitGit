#requires -Version 5
<#
.SYNOPSIS
    UT (Unit-Test) regression tier for the gitgit MVP.

.DESCRIPTION
    Runs `cargo test --quiet`, parses the captured stdout for "test result:
    ok / FAILED" lines, and asserts each of the 9 modules reports
    pass == baseline. Outputs a JSON report at
    `target/regression-logs/ut-<timestamp>.json` and a Markdown digest
    to stdout.

    Baseline counts are read from `scripts/regression-baseline.json`
    (key `ut.expectedTotal` / `ut.perModule`). Drift = failure.

    Exit codes:
      0  all tests pass and counts match baseline
      1  any test failed, or module count drift detected
      2  setup error (cargo missing, baseline missing, etc.)
#>

[CmdletBinding()]
param(
    [switch]$SkipBuild,
    [int]$TimeoutSec = 600
)

$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'lib\regression-common.ps1')

$baselinePath = Join-Path (Get-RepoRoot) 'scripts\regression-baseline.json'
if (-not (Test-Path $baselinePath)) {
    Write-Fail "baseline not found at $baselinePath"
    exit 2
}
$baseline = Get-Content -Raw $baselinePath | ConvertFrom-Json
$expectedTotal    = [int]$baseline.ut.expectedTotal
$expectedPassed   = [int]$baseline.ut.expectedPassed
$expectedFailed   = [int]$baseline.ut.expectedFailed
$expectedIgnored  = [int]$baseline.ut.expectedIgnored
$perModule        = $baseline.ut.perModule

$run = New-RegressionRun -Tier 'ut'

# ── Cargo availability ──────────────────────────────────────────────────────
$cargo = (Get-Command cargo.exe -ErrorAction SilentlyContinue)
if (-not $cargo) {
    Write-Fail 'cargo not found on PATH'
    $run.SetupError = $true
    $report = Write-RegressionReport -Run $run -OutFile (Join-Path (Get-LogRoot) 'ut-setup-error.json')
    exit 2
}

# Per-worktree target dir to avoid shared-cache lock contention with
# other Multica workspaces (see memory: ULYS-100 shared CARGO_TARGET_DIR
# cache pollution). The user-env CARGO_TARGET_DIR=E:\DevCache\cargo\target
# is overridden for THIS script only by clearing it inside the PowerShell
# session — every spawned child inherits the cleared value.
$script:RegressionTargetDir = Join-Path (Get-RepoRoot) 'target-regression'
# CARGO_TARGET_DIR must be a Windows-native path (no /c/Users/... MSYS
# style). If we pass a MSYS-style path, cargo honors it literally and
# creates a literal `C:\c\Users\...` tree under the C: drive root —
# a path-translation bug that surfaces because cargo treats the value
# verbatim and doesn't apply MSYS conversion to env-var paths.
$env:CARGO_TARGET_DIR = $script:RegressionTargetDir
if (-not (Test-Path $script:RegressionTargetDir)) {
    New-Item -ItemType Directory -Path $script:RegressionTargetDir -Force | Out-Null
}
Write-Step "using per-worktree CARGO_TARGET_DIR=$script:RegressionTargetDir (overrides shared cache)"

# ── Run cargo test ──────────────────────────────────────────────────────────
Push-Location (Get-RepoRoot)
try {
    # Invoke cargo via cmd.exe with explicit stdout/stderr redirection.
    # PowerShell's Start-Process + RedirectStandardOutput buffers all of
    # cargo's output until the process exits AND cargo on Windows is
    # picky about line discipline; cmd /c wrapping matches the
    # convention smoke.ps1 uses for `git` and reliably flushes every
    # test result line.
    $argList = @('--color=never', '--no-fail-fast', '--offline')
    $stdoutFile = Join-Path (Get-LogRoot) 'ut-stdout.log'
    $stderrFile = Join-Path (Get-LogRoot) 'ut-stderr.log'
    if (Test-Path $stdoutFile) { Remove-Item $stdoutFile -Force }
    if (Test-Path $stderrFile) { Remove-Item $stderrFile -Force }

    Write-Step "cargo test $($argList -join ' ')  (TimeoutSec=$TimeoutSec, target=$script:RegressionTargetDir)"
    $sw = [System.Diagnostics.Stopwatch]::StartNew()

    # Invoke cargo via bash (git-bash on Windows) so that redirection
    # works natively. PowerShell's Start-Process + RedirectStandardOutput
    # under heavy contention with sibling workspaces (all sharing
    # E:\DevCache\cargo\target and CARGO_HOME) drops output at ~80
    # bytes; bash redirection is reliable and lets cargo finish even
    # while other cargo procs are queued on the package-cache lock.
    $bash = (Get-Command bash.exe -ErrorAction SilentlyContinue)
    if (-not $bash) {
        Write-Fail 'bash.exe not found on PATH (expected from git-bash)'
        $run.SetupError = $true
        $report = Write-RegressionReport -Run $run -OutFile (Join-Path (Get-LogRoot) 'ut-setup-error.json')
        Pop-Location
        exit 2
    }
    $bashPath = $bash.Source
    $repoRootPosix  = (Get-RepoRoot) -replace '\\','/'
    $cargoBinPosix  = ((Get-Command cargo.exe).Source) -replace '\\','/'
    $stdoutPosix    = $stdoutFile -replace '\\','/'
    $stderrPosix    = $stderrFile -replace '\\','/'
    # Write a tiny bash script — much easier to reason about than
    # embedded quoting in a PowerShell here-string that goes through
    # Start-Process argument parsing.
    $shPath = Join-Path $env:TEMP ("gitgit-regression-ut-{0}.sh" -f ([guid]::NewGuid().ToString('N')))
    $exitCodePath = Join-Path (Get-LogRoot) 'ut-exitcode.txt'
    $shBody = "#!/usr/bin/env bash`n" +
              "set -u`n" +
              "cd '$repoRootPosix'`n" +
              "'$cargoBinPosix' test $($argList -join ' ') > '$stdoutPosix' 2> '$stderrPosix'`n" +
              "echo `$? > '$($exitCodePath -replace '\\','/')'`n"
    # NOTE: ``$?`` above is escaped with a backtick to PREVENT PowerShell
    # from interpolating `$?` (which it treats as the boolean success
    # flag and would replace with the literal string "True" before bash
    # ever saw the script).
    Set-Content -Path $shPath -Value $shBody -Encoding ascii -Force
    try {
        $sw = [System.Diagnostics.Stopwatch]::StartNew()
        $proc = Start-Process -FilePath $bashPath `
            -ArgumentList @($shPath) `
            -NoNewWindow `
            -PassThru
        if (-not $proc.WaitForExit($TimeoutSec * 1000)) {
            try { Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue } catch {}
            Write-Fail "cargo test did not finish within ${TimeoutSec}s"
            $run.SetupError = $true
            $report = Write-RegressionReport -Run $run -OutFile (Join-Path (Get-LogRoot) 'ut-timeout.json')
            Pop-Location
            exit 2
        }
        $sw.Stop()
    } finally {
        Remove-Item $shPath -Force -ErrorAction SilentlyContinue
    }
    # Read the exit code that bash wrote to a file (more reliable
    # than $proc.ExitCode under PowerShell 5.1 + bash.exe).
    $exit = -1
    if (Test-Path $exitCodePath) {
        $exitStr = (Get-Content $exitCodePath -Raw -ErrorAction SilentlyContinue).Trim()
        if ($exitStr -match '^\-?\d+$') { $exit = [int]$exitStr }
    }
    # Read the captured output.
    $stdout = ''
    $stderr = ''
    if (Test-Path $stdoutFile) { $stdout = Get-Content $stdoutFile -Raw -ErrorAction SilentlyContinue }
    if (Test-Path $stderrFile) { $stderr = Get-Content $stderrFile -Raw -ErrorAction SilentlyContinue }
    if (-not $stdout) { $stdout = '' }
    if (-not $stderr) { $stderr = '' }
}
finally {
    Pop-Location
}

# ── Parse "test result: ok. N passed; M failed; ..." ───────────────────────
$totals = [ordered]@{ passed=0; failed=0; ignored=0; measured=0; filtered=0 }
$modulePassed = @{}
$moduleFailed = @{}
$moduleTotal  = @{}

foreach ($line in ($stdout -split "`n")) {
    # Aggregate total line, e.g.: "test result: ok. 76 passed; 0 failed; 0 ignored; ..."
    $m = [regex]::Match($line, 'test result:\s*(ok|FAILED)\.\s*(\d+)\s+passed;\s*(\d+)\s+failed;\s*(\d+)\s+ignored')
    if ($m.Success) {
        $totals.passed  = [int]$m.Groups[2].Value
        $totals.failed  = [int]$m.Groups[3].Value
        $totals.ignored = [int]$m.Groups[4].Value
        continue
    }
    # Per-test lines, e.g.: "test server::api::tests::foo ... ok" / "FAILED"
    $m = [regex]::Match($line, '^\s*test\s+([A-Za-z0-9_:]+)\s+\.\.\.\s+(ok|FAILED|ignored)')
    if (-not $m.Success) { continue }
    $fullPath = $m.Groups[1].Value
    $outcome  = $m.Groups[2].Value
    # Take the first two path components ("server::api") to bucket by module.
    $parts = $fullPath -split '::'
    if ($parts.Count -lt 2) { continue }
    $module = "$($parts[0])::$($parts[1])"
    if ($null -eq $moduleTotal[$module]) {
        $moduleTotal[$module]  = 0
        $modulePassed[$module] = 0
        $moduleFailed[$module] = 0
    }
    $moduleTotal[$module]++
    switch ($outcome) {
        'ok'      { $modulePassed[$module]++ }
        'FAILED'  { $moduleFailed[$module]++ }
    }
}

# ── Aggregate assertions ─────────────────────────────────────────────────────
Assert-Equal -Run $run -Name 'ut.total.passed.matches_baseline' `
    -Expected $expectedPassed -Actual $totals.passed `
    -Detail "expected=$expectedPassed (per baseline.ut.expectedPassed)"
Assert-Equal -Run $run -Name 'ut.total.failed.zero' `
    -Expected $expectedFailed -Actual $totals.failed `
    -Detail "any non-zero failure count trips the gate"
Assert-Equal -Run $run -Name 'ut.total.ignored.matches_baseline' `
    -Expected $expectedIgnored -Actual $totals.ignored
Assert-Equal -Run $run -Name 'ut.total_count.matches_sum_of_modules' `
    -Expected $expectedTotal -Actual ($moduleTotal.Values | Measure-Object -Sum).Sum `
    -Detail "expected sum-of-modules=$expectedTotal"

# Per-module parity check.
foreach ($prop in $perModule.PSObject.Properties) {
    $modName = $prop.Name
    $expect  = [int]$prop.Value
    $actual  = [int]($moduleTotal[$modName])
    Assert-Equal -Run $run -Name "ut.module.$modName.count_matches_baseline" `
        -Expected $expect -Actual $actual `
        -Detail "baseline=$expect observed=$actual"
}

# Process exit-code sanity (cargo's exit 0 vs non-zero). We read the
    # exit code from a file bash wrote (more reliable than $proc.ExitCode
    # under PowerShell 5.1 + bash.exe). Defensively coerce null/empty
    # to non-zero so the assertion fails loud rather than silently
    # passing.
    $exitForAssert = if ([string]::IsNullOrEmpty("$exit")) { -1 } else { [int]$exit }
    Assert-Equal -Run $run -Name 'ut.cargo_exit_code.is_zero' `
        -Expected 0 -Actual $exitForAssert `
        -Detail "cargo test exited with $exit"

# ── Report ──────────────────────────────────────────────────────────────────
$outJson = Join-Path (Get-LogRoot) ('ut-{0}.json' -f (Get-Date).ToString('yyyyMMdd-HHmmss'))
$report = Write-RegressionReport -Run $run -OutFile $outJson

Write-Host ''
Write-Host '── UT summary ──'
Write-Host ('  cargo exit          : {0}' -f $exit)
Write-Host ('  passed / failed     : {0} / {1}' -f $totals.passed, $totals.failed)
Write-Host ('  ignored             : {0}' -f $totals.ignored)
Write-Host ('  module totals       :')
foreach ($k in ($moduleTotal.Keys | Sort-Object)) {
    Write-Host ('    {0,-30} passed={1} failed={2}' -f $k, $modulePassed[$k], $moduleFailed[$k])
}
Write-Host ''
Write-Host ('  regression cases    : pass={0} fail={1} skip={2}' -f $run.Pass, $run.Fail, $run.Skip)
Write-Host ('  json report         : {0}' -f $outJson)

if ($run.SetupError) { exit 2 }
if ($run.Fail -gt 0) { exit 1 }
exit 0
