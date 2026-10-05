#requires -Version 7
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
      * No external dependencies beyond a stock CI/dev image:
        cargo, git, curl. The IT tier additionally requires jq, because
        its JSON-shape and array-length assertions have no meaningful
        fallback — see scripts/regression-it.ps1.

    Portability: every path built here uses forward slashes, which are
    accepted by the .NET file APIs on Windows *and* Linux, and no helper
    shells out to cmd.exe. The tier scripts therefore run unchanged on
    Windows pwsh 7 and on ubuntu-latest GitHub runners.
#>

$ErrorActionPreference = 'Stop'

# Repo root = parent of the scripts/ directory that sourced this file.
$script:RegressionRepoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..' | Join-Path -ChildPath '..')).Path
$script:RegressionLogRoot  = Join-Path $script:RegressionRepoRoot 'target/regression-logs'
if (-not (Test-Path $script:RegressionLogRoot)) {
    New-Item -ItemType Directory -Path $script:RegressionLogRoot -Force | Out-Null
}

function Get-RepoRoot { $script:RegressionRepoRoot }
function Get-LogRoot  { $script:RegressionLogRoot }

# ── Cross-platform primitives ───────────────────────────────────────────────

function Test-IsWindows {
    # $IsWindows is an automatic variable on PowerShell 6+; the
    # PlatformID probe is the fallback for Windows PowerShell 5.1.
    if ($null -ne $IsWindows) { return [bool]$IsWindows }
    return ([System.Environment]::OSVersion.Platform -eq [System.PlatformID]::Win32NT)
}

function Get-NullDevicePath {
    # curl's `-o` needs a real discard target; `NUL` only exists on Windows.
    if (Test-IsWindows) { return 'NUL' }
    return '/dev/null'
}

function Get-ScratchTempRoot {
    # $env:TEMP is a Windows-ism. PowerShell on Linux leaves it unset, so
    # fall back to $env:TMP and then to the runtime's temp directory.
    if ($env:TEMP) { return $env:TEMP }
    if ($env:TMP)  { return $env:TMP }
    return ([System.IO.Path]::GetTempPath()).TrimEnd('/', '\')
}

function Get-ToolPath {
    <#
      Resolve an external tool to an absolute path, tolerating the
      Windows `.exe` suffix and the PowerShell `curl` alias (WinPS maps
      `curl` to Invoke-WebRequest, which is not an HTTP client we can
      spawn). Returns $null when the tool is absent.

      Always returns a single scalar string. `Get-Command` can return
      SEVERAL applications for one name (a GitHub ubuntu runner has more
      than one `curl` on PATH), and returning `$cmd.Source` in that case
      yields an array. An array is truthy, so every `if (-not $tool)`
      guard downstream passes, and the failure only surfaces much later
      as an opaque parameter-binding error on whichever parameter
      happened to receive it.
    #>
    param([Parameter(Mandatory)][string]$Name)

    $candidates = @($Name)
    if ($Name -notmatch '\.(exe|cmd|bat)$') { $candidates += "$Name.exe" }
    foreach ($c in $candidates) {
        $cmd = Get-Command $c -CommandType Application -ErrorAction SilentlyContinue |
               Select-Object -First 1
        if ($cmd) { return [string]$cmd.Source }
    }
    return $null
}

function Get-BashPath {
    <#
      Resolve a real POSIX bash, skipping the Windows WSL shim.

      `C:\Windows\System32\bash.exe` is not bash -- it is the WSL launcher.
      It appears early on PATH on any machine with WSL installed, so a
      plain PATH lookup finds it before git-bash and then hands a
      Windows-style path to a Linux shell, which fails in a way that reads
      like a missing file:

        /bin/bash: CUsersleo19AppDataLocalTempgitgit-regression-ut-....sh:
        No such file or directory

      Note what that message shows: the backslashes were eaten, so the
      path is not merely wrong, it is corrupt -- and nothing in the error
      points at WSL.

      Prefers, in order: a bash already on PATH that is not the WSL shim,
      then the git-bash that ships with Git for Windows, then the system
      bash on Linux/macOS. Returns $null when there is genuinely none.
    #>
    $onPath = Get-ToolPath -Name 'bash'
    $isWslShim = $onPath -and
                 ($onPath -match '(?i)\\system32\\bash\.exe$' -or
                  $onPath -match '(?i)\\sysnative\\bash\.exe$')
    if ($onPath -and -not $isWslShim) { return $onPath }

    # The WSL shim is what PATH offered; look past it for a real bash.
    $gitBash = @(
        (Join-Path ${env:ProgramFiles} 'Git\bin\bash.exe'),
        (Join-Path ${env:ProgramFiles} 'Git\usr\bin\bash.exe'),
        '/usr/bin/bash',
        '/bin/bash'
    ) | Where-Object { $_ -and (Test-Path -LiteralPath $_) } | Select-Object -First 1

    if ($gitBash) { return [string]$gitBash }
    return $null
}

function Invoke-External {
    <#
      Run a program with an exact argv vector and capture its output.

      Uses ProcessStartInfo.ArgumentList, which hands each element to the
      OS verbatim. That is what makes this work identically on Windows
      and Linux: no cmd.exe quoting, no PowerShell re-splitting, and no
      risk of a JSON body or a `content-type: application/json` header
      being torn into two arguments. Requires PowerShell 6+.

      Both pipes are drained with ReadToEndAsync *before* WaitForExit so
      a chatty child (cargo) cannot fill one pipe and deadlock.
    #>
    param(
        [Parameter(Mandatory)][string]$FilePath,
        [string[]]$ArgumentList = @(),
        [string]$WorkingDirectory,
        [int]$TimeoutSec = 0
    )

    # Fail loudly and specifically if we were handed something that is not
    # one usable path. A parameter-binding error on [string]$FilePath says
    # only "cannot convert to System.String", which does not tell you
    # whether the value was null, an array, or a CommandInfo -- and this
    # function is called from a dozen places.
    if ([string]::IsNullOrWhiteSpace($FilePath)) {
        throw "Invoke-External: FilePath resolved to an empty value (tool missing on PATH?)"
    }
    if (-not (Test-Path -LiteralPath $FilePath)) {
        $type = $FilePath.GetType().FullName
        throw "Invoke-External: FilePath '$FilePath' (type $type) does not exist on disk"
    }

    $psi = [System.Diagnostics.ProcessStartInfo]::new()
    $psi.FileName               = $FilePath
    $psi.UseShellExecute        = $false
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError  = $true
    $psi.CreateNoWindow         = $true
    if ($WorkingDirectory) { $psi.WorkingDirectory = $WorkingDirectory }
    foreach ($a in $ArgumentList) { [void]$psi.ArgumentList.Add([string]$a) }

    $proc = [System.Diagnostics.Process]::Start($psi)
    $outTask = $proc.StandardOutput.ReadToEndAsync()
    $errTask = $proc.StandardError.ReadToEndAsync()

    $timedOut = $false
    if ($TimeoutSec -gt 0) {
        if (-not $proc.WaitForExit($TimeoutSec * 1000)) {
            $timedOut = $true
            try { $proc.Kill($true) } catch { }
        }
    }
    $proc.WaitForExit()

    [pscustomobject]@{
        ExitCode = if ($timedOut) { -1 } else { $proc.ExitCode }
        StdOut   = $outTask.GetAwaiter().GetResult()
        StdErr   = $errTask.GetAwaiter().GetResult()
        TimedOut = $timedOut
    }
}

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
    #
    # Staleness check: a binary is considered fresh only if it is newer
    # than the **newest file under src/**. If every candidate is stale we
    # return $null so the caller triggers a rebuild.
    #
    # `[FACT]` This used to compare against `src/main.rs` alone, and that
    # is the wrong reference. Measured on 2026-10-05: with
    # `src/server/api.rs` edited and `src/main.rs` untouched, the IT tier
    # reported every assertion green against a binary built *before* the
    # edit — the new auth assertions passed with the auth layer deleted,
    # because the server under test was the previous build. A check that
    # only asks about the crate root is blind to every module, which is
    # where all the logic is. `Cargo.toml` and `Cargo.lock` are included
    # because a manifest change also invalidates the binary.
    #
    # No machine-specific path is searched: a hardcoded shared cache would
    # make the result depend on which developer box the script runs on,
    # and on a CI runner such a path can only ever be a stale hit.
    $root    = Get-RepoRoot
    $exe     = if (Test-IsWindows) { '.exe' } else { '' }

    $srcDir  = Join-Path $root 'src'
    $newest  = @()
    if (Test-Path $srcDir) {
        $newest = @(Get-ChildItem -Path $srcDir -Recurse -File -Include '*.rs' -ErrorAction SilentlyContinue)
    }
    foreach ($manifest in @('Cargo.toml', 'Cargo.lock')) {
        $p = Join-Path $root $manifest
        if (Test-Path $p) { $newest += (Get-Item $p) }
    }
    $newestSrcTime = ($newest | Measure-Object -Property LastWriteTime -Maximum).Maximum

    $candidates = @()
    if ($env:CARGO_TARGET_DIR) {
        $candidates += (Join-Path $env:CARGO_TARGET_DIR "debug/gitgit$exe")
    }
    $candidates += (Join-Path $root "target-regression/debug/gitgit$exe")
    $candidates += (Join-Path $root "target/debug/gitgit$exe")
    foreach ($c in $candidates) {
        if ($c -and (Test-Path $c)) {
            $binTime = (Get-Item $c).LastWriteTime
            if ($newestSrcTime -and ($binTime -lt $newestSrcTime)) {
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
    $d = Join-Path (Get-ScratchTempRoot) "$Prefix-$([guid]::NewGuid().ToString('N'))"
    New-Item -ItemType Directory -Path $d -Force | Out-Null
    return $d
}

function Remove-ScratchRoot {
    param([string]$Path)
    if ($Path -and (Test-Path $Path)) {
        # Remove-Item -Recurse is the only spelling that works on both
        # Windows and Linux; the previous `cmd /c rmdir /S /Q` form was
        # Windows-only and silently no-opped off-Windows.
        Remove-Item -LiteralPath $Path -Recurse -Force -ErrorAction SilentlyContinue
    }
}

function Wait-HttpReady {
    param(
        [Parameter(Mandatory)][string]$Url,
        [int]$MaxAttempts = 50,
        [int]$DelayMs = 200
    )
    $curl = Get-ToolPath -Name 'curl'
    if (-not $curl) {
        Write-Fail 'curl not found on PATH; cannot probe server readiness'
        return $false
    }
    for ($i = 0; $i -lt $MaxAttempts; $i++) {
        Start-Sleep -Milliseconds $DelayMs
        $r = Invoke-External -FilePath $curl -ArgumentList @(
            '-sS', '-o', (Get-NullDevicePath), '-w', '%{http_code}',
            '--max-time', '2', $Url
        )
        if ($r.ExitCode -eq 0 -and $r.StdOut.Trim() -eq '200') { return $true }
    }
    return $false
}

function Format-Duration {
    param([double]$Seconds)
    if ($Seconds -lt 60) { return ('{0:N2}s' -f $Seconds) }
    $ts = [TimeSpan]::FromSeconds($Seconds)
    return ('{0}m{1:N0}s' -f [int]$ts.TotalMinutes, $ts.Seconds)
}
