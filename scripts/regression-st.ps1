#requires -Version 5
<#
.SYNOPSIS
    ST (System Test) regression tier — wraps scripts/smoke.ps1.

.DESCRIPTION
    System tests exercise the full stack end-to-end with a real `git`
    client: HTTP smart protocol negotiate/refs/upload-pack/receive-pack,
    credential handling, branch + tag propagation, merge semantics.

    This script doesn't reimplement smoke.ps1; it delegates to it,
    captures its output, and asserts the expected top-level milestones
    by greppable markers. Adding a new milestone means updating
    smoke.ps1 and adding the marker here — not duplicating logic.

    Baseline assertions are derived from scripts/regression-baseline.json
    (`st.minAssertions`). The script counts positive markers and fails
    if any expected marker is missing.
#>

[CmdletBinding()]
param(
    [string]$Bind = '127.0.0.1:18098',
    [string]$RepoName = 'regression',
    [int]$TimeoutSec = 240
)

$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'lib\regression-common.ps1')

# Per-worktree target dir so smoke.ps1's `git clone http://...` works
# against a binary built against the current source (not the stale
# 8/31 binary on the shared cache). The smoke script uses
# Resolve-Binary from this same helper module.
$script:RegressionTargetDir = Join-Path (Get-RepoRoot) 'target-regression'
$env:CARGO_TARGET_DIR = $script:RegressionTargetDir
if (-not (Test-Path $script:RegressionTargetDir)) {
    New-Item -ItemType Directory -Path $script:RegressionTargetDir -Force | Out-Null
}

$baselinePath = Join-Path (Get-RepoRoot) 'scripts\regression-baseline.json'
if (-not (Test-Path $baselinePath)) {
    Write-Fail "baseline not found at $baselinePath"
    exit 2
}
$baseline = Get-Content -Raw $baselinePath | ConvertFrom-Json
$minAssertions = [int]$baseline.st.minAssertions

$run = New-RegressionRun -Tier 'st'

# ── Locator for smoke.ps1 ──────────────────────────────────────────────────
$smokePath = Join-Path (Get-RepoRoot) 'scripts\smoke.ps1'
if (-not (Test-Path $smokePath)) {
    Write-Fail "smoke.ps1 not found at $smokePath"
    $run.SetupError = $true
    exit 2
}

# ── Run smoke.ps1 ──────────────────────────────────────────────────────────
Write-Step "running pwsh $smokePath -Bind $Bind -RepoName $RepoName"
$logFile = Join-Path (Get-LogRoot) ('smoke-{0}.log' -f (Get-Date).ToString('yyyyMMdd-HHmmss'))
$sw = [System.Diagnostics.Stopwatch]::StartNew()
$proc = Start-Process -FilePath pwsh.exe `
    -ArgumentList @('-NoProfile','-File',$smokePath,'-Bind',$Bind,'-RepoName',$RepoName) `
    -NoNewWindow -PassThru -Wait `
    -RedirectStandardOutput $logFile `
    -RedirectStandardError  "$logFile.err"
$sw.Stop()
$exit = $proc.ExitCode

$logText = if (Test-Path $logFile) { Get-Content -Raw $logFile } else { '' }

# ── Milestone assertions ──────────────────────────────────────────────────
# Each milestone is a phrase smoke.ps1 prints on success of that step.
$milestones = @(
    @{ Name='build';                         Pattern='cargo build --quiet' },
    @{ Name='init_repo';                     Pattern='init-repo' },
    @{ Name='server_ready';                  Pattern='server ready' },
    @{ Name='first_clone';                   Pattern='git clone \(first\)' },
    @{ Name='default_branch';                Pattern='default branch =' },
    @{ Name='initial_commit';                Pattern='initial commit on' },
    @{ Name='push_main';                     Pattern='push -u origin' },
    @{ Name='feature_branch';                Pattern='checkout -b feature' },
    @{ Name='feature_push';                  Pattern='git push.*origin feature|push -u origin feature' },
    @{ Name='second_clone';                  Pattern='git clone \(second\) for merge' },
    @{ Name='merge_commit';                  Pattern='merge commit verified in log' },
    @{ Name='negative_auth_push_rejected';   Pattern='unauthenticated push|received expected auth challenge|push correctly rejected' }
)

foreach ($m in $milestones) {
    Assert-Match -Run $run -Name "st.milestone.$($m.Name)" `
        -Pattern $m.Pattern -Actual $logText `
        -Detail "expected smoke.ps1 output to contain '$($m.Pattern)'"
}

# Final verdict line.
Assert-Match -Run $run -Name 'st.verdict.pass_marker' `
    -Pattern 'gitgit MVP smoke test: PASS' -Actual $logText

# Exit-code parity.
Assert-Equal -Run $run -Name 'st.smoke_exit_code.is_zero' `
    -Expected 0 -Actual $exit `
    -Detail "smoke.ps1 exit=$exit"

# Total milestone count check (defensive against future drift).
$presentMilestones = ($milestones | Where-Object { $logText -match $_.Pattern }).Count
Assert-True -Run $run -Name 'st.milestone_count.meets_baseline' `
    -Condition ($presentMilestones -ge $minAssertions) `
    -Detail "present=$presentMilestones baseline.min=$minAssertions"

# ── Report ─────────────────────────────────────────────────────────────────
$outJson = Join-Path (Get-LogRoot) ('st-{0}.json' -f (Get-Date).ToString('yyyyMMdd-HHmmss'))
$report = Write-RegressionReport -Run $run -OutFile $outJson
Write-Host ''
Write-Host '── ST summary ──'
Write-Host ('  smoke.ps1 exit      : {0}' -f $exit)
Write-Host ('  duration            : {0}' -f (Format-Duration $sw.Elapsed.TotalSeconds))
Write-Host ('  milestones present  : {0} / {1} (baseline.min={2})' -f $presentMilestones, $milestones.Count, $minAssertions)
Write-Host ('  regression cases    : pass={0} fail={1} skip={2}' -f $run.Pass, $run.Fail, $run.Skip)
Write-Host ('  smoke log           : {0}' -f $logFile)
Write-Host ('  json report         : {0}' -f $outJson)
if ($run.SetupError) { exit 2 }
if ($run.Fail -gt 0) { exit 1 }
exit 0
