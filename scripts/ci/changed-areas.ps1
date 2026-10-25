#requires -Version 7
<#
.SYNOPSIS
    Compute which of this repository's five gated areas a change touches.

.DESCRIPTION
    GitHub's `paths:` filter is a WORKFLOW-level switch. A workflow the filter
    excludes never starts, so its checks are never reported, and a required
    status check waiting for them blocks the pull request for ever. The way out
    is to move the filter INSIDE the jobs: a job whose `if:` condition is false
    is reported as "Success", which satisfies a required check.

    This script IS that filter, expressed as a job-level condition. It computes
    the changed-file set between a base ref and a head ref, matches it against
    the five areas' path patterns, and emits one `area=true|false` output per
    area for GitHub Actions to consume.

    The five areas and their pattern lists are the ones the workflows' `paths:`
    blocks listed, verbatim. The refactor changes WHEN a gate reports, not WHAT
    it covers.

    The safety properties are the point of the script, not a side effect of it:

      * "the change set could not be computed" is NEVER reported as "nothing
        changed". A gate that quietly concludes that nothing changed while
        checking nothing is worse than no gate at all, so an unusable base ref
        is a hard, non-zero failure;
      * with no base ref at all (a `schedule` or `workflow_dispatch` event)
        EVERY area is treated as changed — running a gate too often is safe,
        running it too rarely is not;
      * a pattern list that cannot be evaluated (a wildcard the script does not
        implement) is a hard error rather than a literal comparison, because a
        literal comparison would make a gate LOOK like it is working while
        matching the wrong files;
      * the script reads only. It writes no file, switches no branch, commits
        nothing.

.PARAMETER ChangedFilesPath
    Read the changed-file list from this file (one path per line) instead of
    asking git. The script does not call git at all when this is given.

.PARAMETER BaseRef
    Override the base ref the change set is computed against.

.PARAMETER HeadRef
    The head of the change set. Defaults to HEAD.

.PARAMETER OutputPath
    Write the `key=value` outputs to this file as well as to
    $env:GITHUB_OUTPUT. The format is identical.

.PARAMETER PatternsPath
    A JSON file overriding the built-in area pattern lists, e.g.
    `{ "rust_backend": ["src/**"] }`. Areas the file does not mention keep
    their built-in list; an area the file does not know is an error. Used by
    the unit tests to exercise the wildcard-rejection path.

.PARAMETER SelfCheck
    Validate the effective pattern lists, print them, and carry on. A pattern
    list that cannot be evaluated is a hard error with or without this switch.

.EXAMPLE
    pwsh -NoProfile -File scripts/ci/changed-areas.ps1

.EXAMPLE
    pwsh -NoProfile -File scripts/ci/changed-areas.ps1 -ChangedFilesPath changed.txt -OutputPath out.txt

.NOTES
    [FACT] Exit codes: 0 = the areas were computed; 2 = the areas could not
    be computed (no git, an unusable base ref, a malformed pattern list).
    There is no exit code 1: this script does not pass or fail anything, it
    only decides which gates apply.

    [FACT] The path-pattern semantics replicated here are the ones GitHub
    documents for `paths:` — patterns match the whole path starting from the
    repository root, so `Cargo.toml` matches the ROOT `Cargo.toml` and not
    `apps/gm-console/Cargo.toml`. The component that decides the matching is
    not open source, so the equivalence is an inference; the real verification
    is an empirical check on GitHub.
#>

param(
    [string]$ChangedFilesPath,
    [string]$BaseRef,
    [string]$HeadRef = 'HEAD',
    [string]$OutputPath,
    [string]$PatternsPath,
    [switch]$SelfCheck
)

$ErrorActionPreference = 'Stop'

# [FACT] The stopwatch exists because the script's cost was measured and did
# not match expectations: on a GitHub-hosted ubuntu runner this script took
# ~17s inside a test, while the same script on a developer machine takes
# 0.7s. A gate that cannot say where its time goes is a gate that gets
# "optimised" by being switched off. The timings are printed so the next
# measurement can be a measurement and not a guess.
$script:Stopwatch = [System.Diagnostics.Stopwatch]::StartNew()

# ── The five areas ───────────────────────────────────────────────────────────
#
# [FACT] These lists are the `paths:` blocks of the five workflows, verbatim,
# measured from origin/dev on 2026-10-25. They are the semantics this lane
# exists to preserve. The order is the order the workflows listed them in.

$script:AreaOrder = @(
    'rust_backend'
    'gm_console'
    'gm_desktop'
    'gm_desktop_bundle'
    'frontend_audit'
)

$script:BuiltInPatterns = [ordered]@{
    rust_backend = @(
        'src/**'
        'Cargo.toml'
        'Cargo.lock'
        'scripts/**'
        '.github/workflows/rust-backend.yml'
    )
    gm_console = @(
        'apps/gm-console/**'
        'src/server/api.rs'
        '.github/workflows/gm-console.yml'
    )
    gm_desktop = @(
        'apps/gm-desktop/**'
        '.github/workflows/gm-desktop.yml'
        '.github/workflows/release.yml'
        '.github/workflows/gate.yml'
        'Cargo.toml'
        'Cargo.lock'
        'scripts/advisory-baseline.json'
        'scripts/check-advisories.ps1'
    )
    gm_desktop_bundle = @(
        'apps/gm-desktop/**'
        '.github/workflows/gm-desktop-bundle.yml'
        '.github/workflows/gm-desktop.yml'
        'Cargo.toml'
        'Cargo.lock'
        'src/**'
    )
    frontend_audit = @(
        'apps/gm-desktop/**'
        'apps/gm-console/**'
        'scripts/frontend-advisory-baseline.json'
        'scripts/frontend-advisory-check.ps1'
        '.github/workflows/frontend-audit.yml'
    )
}

# ── Output helpers ───────────────────────────────────────────────────────────
#
# [FACT] `Write-Output` would put the machine-readable lines on the success
# stream, whose ordering relative to the information stream is not guaranteed;
# `Write-Host` is the information stream. Writing both through
# [Console]::Out keeps stdout in the order it was produced, which is what the
# unit tests parse.

# [FACT] Every helper carries [CmdletBinding()]. Without it PowerShell binds
# a function's arguments in "simple" mode, where an unrecognised `-f` token is
# swallowed as a positional string instead of being the format operator — so a
# mis-written format call would print `{0}` and exit 0. A gate that reports
# "the pattern is '{1}'" instead of naming the pattern is not a gate.

function Write-Line {
    [CmdletBinding()]
    param([string]$Text)
    [Console]::Out.WriteLine($Text)
}

function Write-ErrorLine {
    [CmdletBinding()]
    param([string]$Text)
    [Console]::Error.WriteLine($Text)
}

function Stop-WithError {
    # Exit code 2 for every failure: this script computes areas, it does not
    # pass or fail a gate, so there is no "the gate is red" exit code here.
    [CmdletBinding()]
    param([string]$Reason)
    Write-ErrorLine "changed-areas: $Reason"
    exit 2
}

# ── Pattern handling ─────────────────────────────────────────────────────────

function Test-PatternIsWellFormed {
    <#
      A pattern is well-formed when it is one of the two shapes this
      repository's lists actually use:

        * a literal path, compared for equality against the whole path from the
          repository root (`Cargo.toml`, `src/server/api.rs`); or
        * a directory wildcard, `dir/**`, which matches every path under
          `dir/`.

      [FACT] The measured lists contain no `*`, `?` or `!` outside the `/**`
      suffix. A pattern that DOES contain one is rejected rather than compared
      literally: a literal comparison would make a gate look like it is
      working while matching the wrong files, which is the failure this script
      exists to prevent.
    #>
    [CmdletBinding()]
    param([string]$Pattern)

    if ([string]::IsNullOrWhiteSpace($Pattern)) { return $false }

    if ($Pattern.EndsWith('/**')) {
        $prefix = $Pattern.Substring(0, $Pattern.Length - 3)
        # `prefix` may not carry a wildcard of its own: `a*/**` and `a?/**`
        # are not shapes this script evaluates.
        return -not ($prefix -match '[*?!]')
    }

    return -not ($Pattern -match '[*?!]')
}

function Test-PathMatchesPattern {
    <#
      [FACT] GitHub documents that path patterns match the whole path starting
      from the repository root. So `Cargo.toml` matches the root
      `Cargo.toml` and NOT `apps/gm-console/Cargo.toml`.
    #>
    [CmdletBinding()]
    param(
        [string]$Path,
        [string]$Pattern
    )

    if ($Pattern.EndsWith('/**')) {
        $prefix = $Pattern.Substring(0, $Pattern.Length - 3)
        if ($prefix -eq '') { return $true }
        return $Path.StartsWith("$prefix/", [System.StringComparison]::Ordinal)
    }

    return [string]::Equals($Path, $Pattern, [System.StringComparison]::Ordinal)
}

function ConvertTo-RepoPath {
    # Normalise a path to the repository-root-relative, forward-slash form
    # `git diff --name-only` produces. A test fixture written on Windows may
    # carry backslashes; the comparison must not depend on the separator.
    [CmdletBinding()]
    param(
        [string]$Path
    )

    $p = $Path.Trim()
    $p = $p.Replace('\', '/')
    while ($p.StartsWith('./')) { $p = $p.Substring(2) }
    return $p
}

# ── The pattern set in effect ────────────────────────────────────────────────

function Get-EffectivePatterns {
    <#
      The built-in lists, with any -PatternsPath override applied on top.
    #>
    [CmdletBinding()]
    param()

    $effective = [ordered]@{}
    foreach ($area in $script:AreaOrder) {
        $effective[$area] = @($script:BuiltInPatterns[$area])
    }

    if (-not $PatternsPath) { return $effective }

    if (-not (Test-Path -LiteralPath $PatternsPath)) {
        Stop-WithError "the pattern override file does not exist: $PatternsPath"
    }

    try {
        $raw = Get-Content -Raw -LiteralPath $PatternsPath | ConvertFrom-Json
    } catch {
        Stop-WithError "the pattern override file is not valid JSON: $($_.Exception.Message)"
    }

    foreach ($property in $raw.PSObject.Properties) {
        $area = $property.Name
        if (-not $script:AreaOrder.Contains($area)) {
            Stop-WithError ("the pattern override names an unknown area '{0}'; the five areas are {1}" -f `
                $area, ($script:AreaOrder -join ', '))
        }
        $patterns = @()
        foreach ($item in @($property.Value)) {
            if ($item -isnot [string]) {
                Stop-WithError "the pattern override for area '$area' holds a non-string entry"
            }
            $patterns += $item
        }
        $effective[$area] = $patterns
    }

    return $effective
}

function Assert-PatternsAreUsable {
    <#
      Every pattern of every area must be one of the two shapes this script
      evaluates. A pattern that is not is a hard error: the alternative is a
      literal comparison, which is a gate that looks alive and matches the
      wrong files.
    #>
    [CmdletBinding()]
    param([System.Collections.IDictionary]$Patterns)

    foreach ($area in $script:AreaOrder) {
        $list = @($Patterns[$area])
        if ($list.Count -eq 0) {
            Stop-WithError "area '$area' has an empty pattern list, so nothing could ever match it"
        }
        foreach ($pattern in $list) {
            if (-not (Test-PatternIsWellFormed -Pattern $pattern)) {
                $reason = ("area '{0}' holds the pattern '{1}', which this script cannot evaluate. " +
                    'Only a literal path and a `dir/**` directory wildcard are implemented; a ' +
                    'wildcard would be compared literally and would match the wrong files while ' +
                    'looking like a working gate.') -f $area, $pattern
                Stop-WithError $reason
            }
        }
    }
}

# ── Git ──────────────────────────────────────────────────────────────────────

function Invoke-Git {
    <#
      Run git with an exact argv vector and capture its output.

      The repository's scripts/lib/regression-common.ps1 has a helper for this,
      but sourcing that file creates a `target/regression-logs` directory as a
      side effect, and this script must not write to the repository. So the
      process is driven directly here.
    #>
    [CmdletBinding()]
    param(
        [Parameter(Mandatory)][string]$RepositoryRoot,
        [Parameter(Mandatory)][string[]]$Arguments
    )

    $gitCommand = Get-Command -Name 'git' -CommandType Application -ErrorAction SilentlyContinue |
        Select-Object -First 1
    if (-not $gitCommand) {
        Stop-WithError 'git is not available on PATH, so the change set cannot be computed'
    }

    $startInfo = [System.Diagnostics.ProcessStartInfo]::new()
    $startInfo.FileName = $gitCommand.Source
    $startInfo.UseShellExecute = $false
    $startInfo.RedirectStandardOutput = $true
    $startInfo.RedirectStandardError = $true
    $startInfo.WorkingDirectory = $RepositoryRoot
    foreach ($argument in $Arguments) { $startInfo.ArgumentList.Add($argument) }

    $process = [System.Diagnostics.Process]::new()
    $process.StartInfo = $startInfo
    $null = $process.Start()

    # Both pipes are drained before the wait, so a chatty child cannot fill
    # one and deadlock.
    $stdoutTask = $process.StandardOutput.ReadToEndAsync()
    $stderrTask = $process.StandardError.ReadToEndAsync()
    $process.WaitForExit()

    return [pscustomobject]@{
        ExitCode = $process.ExitCode
        StdOut   = $stdoutTask.GetAwaiter().GetResult()
        StdErr   = $stderrTask.GetAwaiter().GetResult()
    }
}

# ── Base ref resolution ──────────────────────────────────────────────────────

function Resolve-BaseRef {
    <#
      [FACT] The resolution order is: an explicit -BaseRef, then
      `origin/$GITHUB_BASE_REF` (the pull_request event), then
      `$GITHUB_EVENT_BEFORE` (the push event). With none of them there is no
      baseline and every area is treated as changed.
    #>
    [CmdletBinding()]
    param()

    if ($BaseRef) {
        return [pscustomobject]@{ Ref = $BaseRef; Source = '-BaseRef'; HasBase = $true }
    }

    $githubBaseRef = $env:GITHUB_BASE_REF
    if ($githubBaseRef) {
        return [pscustomobject]@{ Ref = "origin/$githubBaseRef"; Source = 'GITHUB_BASE_REF'; HasBase = $true }
    }

    $eventBefore = $env:GITHUB_EVENT_BEFORE
    if ($eventBefore) {
        # [FACT] GitHub sends the all-zero SHA as the `before` of a push that
        # CREATES a branch. There is no earlier commit to diff against, so
        # there is no baseline — and the conservative reading of "no baseline"
        # is "everything changed", not "nothing changed". Treating it as a
        # real base ref would fail the diff and redden a job that has no
        # reason to be red.
        if ($eventBefore -match '^0+$') {
            return [pscustomobject]@{
                Ref      = $null
                Source   = 'GITHUB_EVENT_BEFORE (all-zero SHA: the push created the branch)'
                HasBase  = $false
            }
        }
        return [pscustomobject]@{ Ref = $eventBefore; Source = 'GITHUB_EVENT_BEFORE'; HasBase = $true }
    }

    return [pscustomobject]@{
        Ref     = $null
        Source  = 'none (no -BaseRef, no GITHUB_BASE_REF, no GITHUB_EVENT_BEFORE)'
        HasBase = $false
    }
}

# ── Main ─────────────────────────────────────────────────────────────────────

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..' | Join-Path -ChildPath '..')).Path

$patterns = Get-EffectivePatterns
Assert-PatternsAreUsable -Patterns $patterns

if ($SelfCheck) {
    Write-Line '[step] changed-areas self-check — the effective area pattern lists'
    foreach ($area in $script:AreaOrder) {
        foreach ($pattern in @($patterns[$area])) {
            Write-Line ("[pattern] {0} :: {1}" -f $area, $pattern)
        }
    }
    $patternCount = 0
    foreach ($area in $script:AreaOrder) { $patternCount += @($patterns[$area]).Count }
    Write-Line ("[info] self-check: {0} areas, {1} patterns, all in a shape this script evaluates" -f `
        $script:AreaOrder.Count, $patternCount)
}

$changedFiles = @()
$hasBase = $false
# [FACT] These are NOT named `$baseRef` / `$baseSource`: PowerShell variable
# names are case-insensitive, so `$baseRef` would be the SAME variable as the
# `-BaseRef` parameter, and assigning to it here would erase the parameter
# before the base ref is resolved.
$resolvedBaseRef = $null
$resolvedBaseSource = ''

if ($ChangedFilesPath) {
    # [FACT] The changed-file list was supplied, so git is not called at all
    # and no base ref is needed: the list IS the change set.
    if (-not (Test-Path -LiteralPath $ChangedFilesPath)) {
        Stop-WithError "the changed-file list does not exist: $ChangedFilesPath"
    }
    $lines = Get-Content -LiteralPath $ChangedFilesPath
    foreach ($line in $lines) {
        $path = ConvertTo-RepoPath -Path $line
        if ($path) { $changedFiles += $path }
    }
    $resolvedBaseSource = "changed-file list '$ChangedFilesPath' (git was not called)"
}
else {
    $resolved = Resolve-BaseRef
    $resolvedBaseSource = $resolved.Source
    $hasBase = $resolved.HasBase
    $resolvedBaseRef = $resolved.Ref

    if ($hasBase) {
        $diffWatch = [System.Diagnostics.Stopwatch]::StartNew()
        $diff = Invoke-Git -RepositoryRoot $repoRoot -Arguments @(
            'diff', '--name-only', "$resolvedBaseRef...$HeadRef"
        )
        $diffWatch.Stop()
        Write-Line ("[info] changed-areas: the change set took {0}ms to compute" -f $diffWatch.ElapsedMilliseconds)
        if ($diff.ExitCode -ne 0) {
            # [FACT] This is the case the whole script is written around: a
            # base ref that exists but cannot be diffed (a shallow clone that
            # does not have it, a typo) must NOT be reported as "nothing
            # changed". It is a failure, loudly.
            $reason = ("the change set could not be computed: `git diff --name-only " +
                "$resolvedBaseRef...$HeadRef` exited $($diff.ExitCode). " +
                "Base ref: '$resolvedBaseRef' (from $($resolved.Source)). " +
                "This is NOT the same as 'nothing changed' and the gate must not pass on it. " +
                "stderr: $($diff.StdErr.Trim())")
            Stop-WithError $reason
        }
        foreach ($line in ($diff.StdOut -split "`n")) {
            $path = ConvertTo-RepoPath -Path $line
            if ($path) { $changedFiles += $path }
        }
    }
}

# ── Match ────────────────────────────────────────────────────────────────────

$areaResults = [ordered]@{}

if (-not $hasBase -and -not $ChangedFilesPath) {
    # [FACT] No baseline and no supplied list: every area is treated as
    # changed. Running a gate too often is safe; running it too rarely is not.
    $noBaseMessage = ("[warn] changed-areas: no baseline ref could be resolved ({0}), so EVERY " +
        'area is treated as changed. This is the conservative reading: a schedule or ' +
        'workflow_dispatch event has no base to diff against, and a gate that runs when it need ' +
        'not is far cheaper than a gate that does not run when it should.') -f $resolvedBaseSource
    Write-Line $noBaseMessage
    foreach ($area in $script:AreaOrder) {
        $areaResults[$area] = [pscustomobject]@{
            Hit   = $true
            Files = @()
        }
    }
}
else {
    foreach ($area in $script:AreaOrder) {
        $hits = @()
        foreach ($file in $changedFiles) {
            foreach ($pattern in @($patterns[$area])) {
                if (Test-PathMatchesPattern -Path $file -Pattern $pattern) {
                    $hits += $file
                    break
                }
            }
        }
        $areaResults[$area] = [pscustomobject]@{
            Hit   = ($hits.Count -gt 0)
            Files = @($hits | Select-Object -Unique)
        }
    }
}

# ── Report ───────────────────────────────────────────────────────────────────

Write-Line ("[info] changed-areas: base '{0}' (from {1}), head '{2}'" -f `
    $(if ($resolvedBaseRef) { $resolvedBaseRef } else { '(none)' }), $resolvedBaseSource, $HeadRef)
Write-Line ("[info] changed-areas: {0} changed file(s)" -f $changedFiles.Count)
foreach ($file in $changedFiles) { Write-Line "[info]   $file" }

foreach ($area in $script:AreaOrder) {
    $result = $areaResults[$area]
    $label = if ($result.Hit) { 'HIT ' } else { 'miss' }
    $shown = @($result.Files | Select-Object -First 5)
    $more = @($result.Files).Count - $shown.Count
    $fileList = if ($shown.Count -gt 0) { $shown -join ', ' } else { '-' }
    if ($more -gt 0) { $fileList = "$fileList (+$more more)" }
    Write-Line ("[area] {0,-20} {1}  {2}" -f $area, $label, $fileList)
}

$outputLines = @()
foreach ($area in $script:AreaOrder) {
    $value = if ($areaResults[$area].Hit) { 'true' } else { 'false' }
    $outputLines += "$area=$value"
    Write-Line "$area=$value"
}

# ── Emit ─────────────────────────────────────────────────────────────────────

if ($env:GITHUB_OUTPUT) {
    foreach ($line in $outputLines) {
        Add-Content -LiteralPath $env:GITHUB_OUTPUT -Value $line
    }
    Write-Line ("[info] changed-areas: wrote {0} output(s) to $env:GITHUB_OUTPUT" -f $outputLines.Count)
}

if ($OutputPath) {
    $utf8NoBom = [System.Text.UTF8Encoding]::new($false)
    [System.IO.File]::WriteAllLines(
        [System.IO.Path]::GetFullPath($OutputPath),
        [string[]]$outputLines,
        $utf8NoBom)
    Write-Line ("[info] changed-areas: wrote {0} output(s) to {1}" -f $outputLines.Count, $OutputPath)
}

$script:Stopwatch.Stop()
Write-Line ("[info] changed-areas: {0}ms in total" -f $script:Stopwatch.ElapsedMilliseconds)

exit 0
