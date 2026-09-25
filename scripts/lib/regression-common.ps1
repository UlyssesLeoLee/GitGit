#requires -Version 5
<#
.SYNOPSIS
    Common helpers shared by all GitGit regression-tier scripts (UT/IT/ST).

.DESCRIPTION
    Provides:
      * Repo path resolution (so callers don't need to know $RepoRoot).
      * Result accumulator (pass/fail/skip counters + per-case rows).
      * Color-agnostic step / pass / fail / warn printers.
      * Exit-code contract: 0 = all-pass, 1 = any-fail, 2 = setup-error.

    Conventions:
      * Every script dot-sources this file with `. $PSScriptRoot\lib\regression-common.ps1`.
      * Callers use `New-RegressionRun -Tier <name>`, then
        `Assert-True / Assert-Equal / Assert-Match / Assert-StatusCode`,
        then `Write-RegressionReport` and `exit` the return code.
      * No external dependencies beyond what is on a stock Windows +
        git-bash dev box: cargo, git, curl, jq.
#>

$ErrorActionPreference = 'Stop'

# Repo root = parent of the scripts/ directory that sourced this file.
$script:RegressionRepoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$script:RegressionLogRoot  = Join-Path $script:RegressionRepoRoot 'target\regression-logs'
if (-not (Test-Path $script:RegressionLogRoot)) {
    New-Item -ItemType Directory -Path $script:RegressionLogRoot -Force | Out-Null
}

function Get-RepoRoot { $script:RegressionRepoRoot }
function Get-LogRoot  { $script:RegressionLogRoot }

function Write-Step {
    param([string]$Message)
    Write-Host "[step] $Message"
}

function Write-Pass {
    param([string]$Message)
    Write-Host "[pass] $Message" -ForegroundColor Green
}

function Write-Fail {
    param([string]$Message)
    Write-Host "[fail] $Message" -ForegroundColor Red
}

function Write-Warn {
    param([string]$Message)
    Write-Host "[warn] $Message" -ForegroundColor Yellow
}

function Write-Info {
    param([string]$Message)
    Write-Host "[info] $Message"
}

# ── Result accumulator ────────────────────────────────────────────────────────

function New-RegressionRun {
    param([Parameter(Mandatory)][string]$Tier)
    return [pscustomobject]@{
        Tier    = $Tier
        Started = (Get-Date).ToString('o')
        Cases   = New-Object System.Collections.Generic.List[object]
        Pass    = 0
        Fail    = 0
        Skip    = 0
        SetupError = $false
    }
}

function Add-RegressionCase {
    param(
        [Parameter(Mandatory)][psobject]$Run,
        [Parameter(Mandatory)][string]$Name,
        [Parameter(Mandatory)][ValidateSet('pass','fail','skip')][string]$Outcome,
        [string]$Detail = '',
        [double]$DurationSec = 0
    )
    $Run.Cases.Add([pscustomobject]@{
        name        = $Name
        outcome     = $Outcome
        detail      = $Detail
        durationSec = [math]::Round($DurationSec, 3)
    }) | Out-Null
    switch ($Outcome) {
        'pass' { $Run.Pass++ }
        'fail' { $Run.Fail++ }
        'skip' { $Run.Skip++ }
    }
}

# ── Assertion helpers ─────────────────────────────────────────────────────────

function Assert-True {
    param(
        [Parameter(Mandatory)][psobject]$Run,
        [Parameter(Mandatory)][string]$Name,
        [bool]$Condition,
        [string]$Detail = ''
    )
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    if ($Condition) {
        Add-RegressionCase -Run $Run -Name $Name -Outcome pass -Detail $Detail -DurationSec $sw.Elapsed.TotalSeconds
        Write-Pass $Name
    } else {
        $msg = if ($Detail) { $Detail } else { 'condition was false' }
        Add-RegressionCase -Run $Run -Name $Name -Outcome fail -Detail $msg -DurationSec $sw.Elapsed.TotalSeconds
        Write-Fail "$Name :: $msg"
    }
}

function Assert-Equal {
    param(
        [Parameter(Mandatory)][psobject]$Run,
        [Parameter(Mandatory)][string]$Name,
        $Expected,
        $Actual,
        [string]$Detail = ''
    )
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    $same = ($Expected -eq $Actual)
    $msg  = if ($Detail) { $Detail }
            elseif ($same) { "expected=$Expected actual=$Actual" }
            else           { "expected=$Expected actual=$Actual" }
    if ($same) {
        Add-RegressionCase -Run $Run -Name $Name -Outcome pass -Detail "expected=$Expected actual=$Actual" -DurationSec $sw.Elapsed.TotalSeconds
        Write-Pass $Name
    } else {
        Add-RegressionCase -Run $Run -Name $Name -Outcome fail -Detail "expected=$Expected actual=$Actual :: $Detail" -DurationSec $sw.Elapsed.TotalSeconds
        Write-Fail "$Name :: expected=<$Expected> actual=<$Actual>"
    }
}

function Assert-Match {
    param(
        [Parameter(Mandatory)][psobject]$Run,
        [Parameter(Mandatory)][string]$Name,
        [Parameter(Mandatory)][string]$Pattern,
        [string]$Actual,
        [string]$Detail = ''
    )
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    if ([string]::IsNullOrEmpty($Actual)) {
        Add-RegressionCase -Run $Run -Name $Name -Outcome fail -Detail 'actual was empty/null' -DurationSec $sw.Elapsed.TotalSeconds
        Write-Fail "$Name :: actual was empty"
        return
    }
    if ($Actual -match $Pattern) {
        Add-RegressionCase -Run $Run -Name $Name -Outcome pass -Detail "pattern=$Pattern matched" -DurationSec $sw.Elapsed.TotalSeconds
        Write-Pass $Name
    } else {
        Add-RegressionCase -Run $Run -Name $Name -Outcome fail -Detail "pattern=$Pattern did not match actual=<$Actual>" -DurationSec $sw.Elapsed.TotalSeconds
        Write-Fail "$Name :: pattern <$Pattern> did not match <$Actual>"
    }
}

function Assert-StatusCode {
    param(
        [Parameter(Mandatory)][psobject]$Run,
        [Parameter(Mandatory)][string]$Name,
        [int]$Expected,
        [int]$Actual,
        [string]$Detail = ''
    )
    Assert-Equal -Run $Run -Name $Name -Expected $Expected -Actual $Actual -Detail $Detail
}

function Skip-Case {
    param(
        [Parameter(Mandatory)][psobject]$Run,
        [Parameter(Mandatory)][string]$Name,
        [string]$Reason = 'skipped'
    )
    Add-RegressionCase -Run $Run -Name $Name -Outcome skip -Detail $Reason
    Write-Warn "$Name :: $Reason"
}

# ── Report writer ─────────────────────────────────────────────────────────────

function Write-RegressionReport {
    param(
        [Parameter(Mandatory)][psobject]$Run,
        [Parameter(Mandatory)][string]$OutFile
    )
    $ended = (Get-Date).ToString('o')
    $report = [pscustomobject]@{
        tier     = $Run.Tier
        started  = $Run.Started
        ended    = $ended
        pass     = $Run.Pass
        fail     = $Run.Fail
        skip     = $Run.Skip
        total    = $Run.Pass + $Run.Fail + $Run.Skip
        cases    = $Run.Cases
    }
    $json = $report | ConvertTo-Json -Depth 6
    Set-Content -Path $OutFile -Value $json -Encoding utf8
    return $report
}

function Exit-RegressionRun {
    param(
        [Parameter(Mandatory)][psobject]$Run,
        [int]$OutFile
    )
    if ($Run.SetupError) { exit 2 }
    if ($Run.Fail -gt 0) { exit 1 }
    exit 0
}

# ── Misc helpers ──────────────────────────────────────────────────────────────

function Resolve-Binary {
    # Locate the gitgit binary. Prefer the per-worktree target dir
    # (target-regression/) so we always run against the current source.
    # Fall back to the shared dev cache (E:\DevCache\cargo\target) if
    # a per-worktree build hasn't been done.
    #
    # Staleness check: a binary in target-regression/ is considered
    # fresh only if it is newer than src/main.rs. If it's stale, we
    # return $null so the caller triggers a rebuild.
    $root = Get-RepoRoot
    $mainSrc = Join-Path $root 'src\main.rs'
    $mainSrcTime = (Get-Item $mainSrc -ErrorAction SilentlyContinue).LastWriteTime

    $candidates = @()
    if ($env:CARGO_TARGET_DIR) {
        $candidates += (Join-Path $env:CARGO_TARGET_DIR 'debug\gitgit.exe')
    }
    $candidates += (Join-Path $root 'target-regression\debug\gitgit.exe')
    $candidates += (Join-Path $root 'target\debug\gitgit.exe')
    $candidates += 'E:\DevCache\cargo\target\debug\gitgit.exe'
    foreach ($c in $candidates) {
        if ($c -and (Test-Path $c)) {
            $binTime = (Get-Item $c).LastWriteTime
            if ($mainSrcTime -and ($binTime -lt $mainSrcTime)) {
                # Stale — skip this candidate so the caller rebuilds.
                continue
            }
            return $c
        }
    }
    return $null
}

function New-ScratchRoot {
    param([string]$Prefix = 'gitgit-regression')
    $d = Join-Path $env:TEMP "$Prefix-$([guid]::NewGuid().ToString('N'))"
    New-Item -ItemType Directory -Path $d -Force | Out-Null
    return $d
}

function Remove-ScratchRoot {
    param([string]$Path)
    if ($Path -and (Test-Path $Path)) {
        cmd /c "rmdir /S /Q `"$Path`"" 2>$null | Out-Null
    }
}

function Wait-HttpReady {
    param(
        [Parameter(Mandatory)][string]$Url,
        [int]$MaxAttempts = 50,
        [int]$DelayMs = 200
    )
    for ($i = 0; $i -lt $MaxAttempts; $i++) {
        Start-Sleep -Milliseconds $DelayMs
        $code = cmd /c "C:\Windows\System32\curl.exe -sS -o NUL -w ""%{http_code}"" --max-time 2 ""$Url""" 2>$null
        if ($code -eq '200') { return $true }
    }
    return $false
}

function Format-Duration {
    param([double]$Seconds)
    if ($Seconds -lt 60) { return ('{0:N2}s' -f $Seconds) }
    $ts = [TimeSpan]::FromSeconds($Seconds)
    return ('{0}m{1:N0}s' -f [int]$ts.TotalMinutes, $ts.Seconds)
}
