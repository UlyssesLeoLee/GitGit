#requires -Version 5
<#
.SYNOPSIS
    Regression orchestrator — runs the UT, IT, and ST tiers in order
    and produces a unified pass/fail report.

.DESCRIPTION
    Usage:
      pwsh scripts/regression.ps1                 # full suite
      pwsh scripts/regression.ps1 -SkipBuild       # assume binary already built
      pwsh scripts/regression.ps1 -Tiers ut,it     # subset
      pwsh scripts/regression.ps1 -ContinueOnFail  # don't abort on first failure
      pwsh scripts/regression.ps1 -Utc             # UTC timestamps in summary

    Each tier script writes its own JSON report to
    `target/regression-logs/<tier>-<timestamp>.json`. This
    orchestrator additionally writes a merged summary at
    `target/regression-logs/regression-summary-<timestamp>.json`
    and prints a colored table to stdout.

    Exit codes:
      0  every tier passed
      1  any tier reported failures
      2  any tier had a setup error
#>

[CmdletBinding()]
param(
    [string[]]$Tiers = @('ut','it','st'),
    [switch]$SkipBuild,
    [switch]$ContinueOnFail,
    [switch]$Utc
)

$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'lib\regression-common.ps1')

$repoRoot = Get-RepoRoot
$logRoot  = Get-LogRoot
$stamp    = if ($Utc) { (Get-Date).ToUniversalTime().ToString('yyyyMMdd-HHmmssZ') }
            else      { (Get-Date).ToString('yyyyMMdd-HHmmss') }

Write-Host '═══════════════════════════════════════════════════════════════' -ForegroundColor Cyan
Write-Host '  GitGit regression orchestrator' -ForegroundColor Cyan
Write-Host ('  repo     : {0}' -f $repoRoot)
Write-Host ('  tiers    : {0}' -f ($Tiers -join ', '))
Write-Host ('  log root : {0}' -f $logRoot)
Write-Host '═══════════════════════════════════════════════════════════════' -ForegroundColor Cyan
Write-Host ''

$results = @()
$globalFail = $false
$globalSetup = $false

foreach ($tier in $Tiers) {
    $scriptPath = Join-Path $PSScriptRoot "regression-$tier.ps1"
    if (-not (Test-Path $scriptPath)) {
        Write-Host "[orchestrator] tier script not found: $scriptPath" -ForegroundColor Red
        $results += [pscustomobject]@{ tier=$tier; exit=2; pass=0; fail=0; skip=0; logFile=$null; durationSec=0 }
        $globalSetup = $true
        continue
    }
    Write-Host ""
    Write-Host "══════════ $tier ══════════" -ForegroundColor Cyan
    $argsForTier = @()
    if ($SkipBuild) { $argsForTier += '-SkipBuild' }
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    # IMPORTANT: $scriptPath may contain spaces (e.g.
    # "multica_workspaces_desktop-api.multica.ai"). Pass it as a
    # single argv element via the array splat — embedding it
    # directly in the `&` call string causes PowerShell to split
    # the path at every space.
    $pwshArgs = @('-NoProfile', '-File', $scriptPath) + $argsForTier
    # Honor DEBUG_REGRESSION_* env from the orchestrator's parent.
    # PowerShell's `& pwsh.exe @pwshArgs` does NOT inherit env vars
    # from the orchestrator's session unless we explicitly forward
    # the ones the tier scripts consult.
    foreach ($k in 'DEBUG_REGRESSION_IT','DEBUG_REGRESSION_UT','DEBUG_REGRESSION_ST') {
        $v = [Environment]::GetEnvironmentVariable($k)
        if ($v) { $pwshArgs += @("-Env:$k", $v) }
    }
    & pwsh.exe @pwshArgs
    $exit = $LASTEXITCODE
    $sw.Stop()

    # Find the newest report for this tier.
    $reportFile = Get-ChildItem -Path $logRoot -Filter "$tier-*.json" -ErrorAction SilentlyContinue |
        Sort-Object LastWriteTime -Descending | Select-Object -First 1
    $reportData = if ($reportFile) { Get-Content -Raw $reportFile.FullName | ConvertFrom-Json } else { $null }

    $passCount = if ($reportData) { [int]$reportData.pass } else { 0 }
    $failCount = if ($reportData) { [int]$reportData.fail } else { 0 }
    $skipCount = if ($reportData) { [int]$reportData.skip } else { 0 }

    $results += [pscustomobject]@{
        tier        = $tier
        exit        = $exit
        pass        = $passCount
        fail        = $failCount
        skip        = $skipCount
        logFile     = if ($reportFile) { $reportFile.FullName } else { $null }
        durationSec = [math]::Round($sw.Elapsed.TotalSeconds, 2)
    }

    if ($exit -eq 2) {
        $globalSetup = $true
        $globalFail  = $true
    } elseif ($exit -ne 0) {
        $globalFail = $true
    }

    if ($exit -eq 0) {
        Write-Host ("[orchestrator] {0} PASS  ({1} cases)" -f $tier, ($passCount+$failCount+$skipCount)) -ForegroundColor Green
    } elseif ($exit -eq 2) {
        Write-Host ("[orchestrator] {0} SETUP-ERROR" -f $tier) -ForegroundColor Red
    } else {
        Write-Host ("[orchestrator] {0} FAIL  (pass={1} fail={2} skip={3})" -f $tier, $passCount, $failCount, $skipCount) -ForegroundColor Red
    }

    if (-not $ContinueOnFail -and $globalFail) {
        Write-Host '[orchestrator] aborting on first failure (use -ContinueOnFail to keep going)' -ForegroundColor Yellow
        break
    }
}

# ── Final summary ───────────────────────────────────────────────────────────
Write-Host ''
Write-Host '═══════════════════════════════════════════════════════════════' -ForegroundColor Cyan
Write-Host '  Final summary' -ForegroundColor Cyan
Write-Host '═══════════════════════════════════════════════════════════════' -ForegroundColor Cyan
$totalPass = ($results | Measure-Object -Property pass -Sum).Sum
$totalFail = ($results | Measure-Object -Property fail -Sum).Sum
$totalSkip = ($results | Measure-Object -Property skip -Sum).Sum
$totalDur  = ($results | Measure-Object -Property durationSec -Sum).Sum

$results | ForEach-Object {
    $color = if ($_.exit -eq 0) { 'Green' } elseif ($_.exit -eq 2) { 'Yellow' } else { 'Red' }
    $verdict = if ($_.exit -eq 0) { 'PASS' } elseif ($_.exit -eq 2) { 'SETUP' } else { 'FAIL' }
    Write-Host ("  {0,-5} {1,-4} pass={2,4} fail={3,4} skip={4,4} duration={5,8}s  log={6}" -f `
        $_.tier, $verdict, $_.pass, $_.fail, $_.skip, $_.durationSec, $_.logFile) -ForegroundColor $color
}

Write-Host ('  TOTAL           pass={0,4} fail={1,4} skip={2,4} duration={3,8}s' -f `
    $totalPass, $totalFail, $totalSkip, $totalDur)
Write-Host ''

# Persist merged summary.
$summary = [pscustomobject]@{
    schemaVersion = '1.0'
    timestamp     = if ($Utc) { (Get-Date).ToUniversalTime().ToString('o') } else { (Get-Date).ToString('o') }
    repoRoot      = $repoRoot
    tiers         = $results
    totals        = [pscustomobject]@{ pass=$totalPass; fail=$totalFail; skip=$totalSkip; durationSec=$totalDur }
    verdict       = if ($globalSetup) { 'setup-error' } elseif ($globalFail) { 'fail' } else { 'pass' }
}
$summaryPath = Join-Path $logRoot "regression-summary-$stamp.json"
$summary | ConvertTo-Json -Depth 5 | Set-Content -Path $summaryPath -Encoding utf8
Write-Host ('  summary json : {0}' -f $summaryPath) -ForegroundColor Cyan

if ($globalSetup) { exit 2 }
if ($globalFail)  { exit 1 }
exit 0
