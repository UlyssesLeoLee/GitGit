#requires -Version 7
<#
.SYNOPSIS
    Refuse a PowerShell script that assigns to a read-only automatic variable.

.DESCRIPTION
    PowerShell's automatic variables include a set that cannot be
    overwritten. Assigning to one of them throws:

        无法覆盖变量 Host ，因为它是只读变量或常量。

    A script that runs under `$ErrorActionPreference = 'Stop'` therefore
    aborts at the assignment — and if it has already started a child
    process, the abort leaves that child running with nobody supervising
    it. This is not hypothetical: `deploy/objectstore/start-objectstore.ps1`
    assigned to `$host`, and the script aborted before its readiness probe
    and before its foreground wait, leaving the object store process behind.

    The repository has no PowerShell linter, so nothing else would have
    caught it. This check is that something.

    [FACT] The list of read-only automatic variables is **not copied into
    this script**. It is **measured, on every run**, by trying to assign to
    each candidate and seeing which ones this PowerShell refuses. A copied
    list is a list that drifts when PowerShell changes; a measurement is
    not.

    [FACT] The measurement uses a **direct assignment** (`$name = ...`
    compiled into a scriptblock), not `Set-Variable`. The two do not agree:
    `Set-Variable` creates a new variable in the current scope, while a
    direct assignment hits the read-only flag on the automatic variable and
    throws. Measuring with the wrong one would miss the exact case this
    check exists for.

.PARAMETER RepoRoot
    The repository to scan. Default: the repository this script lives in.

.EXAMPLE
    pwsh -NoProfile -File scripts/ci/check-powershell-automatic-variables.ps1
#>

[CmdletBinding()]
param(
    [string]$RepoRoot = ''
)

$ErrorActionPreference = 'Stop'

if (-not $RepoRoot) {
    $RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
}

# ── Measure which automatic variables this PowerShell refuses to overwrite ──
# [FACT] The candidate list is deliberately generous: it is a superset of
# what any given PowerShell protects, and the measurement decides which of
# them actually matter here.
$candidates = @(
    'Host', 'PID', 'HOME', 'Profile', 'PWD', 'PSVersionTable', 'PSCulture',
    'PSUICulture', 'PSEdition', 'PSHome', 'PSCommandPath', 'PSScriptRoot',
    'PSCommandArgs', 'PSBoundParameters', 'PSCmdlet', 'MyInvocation',
    'ExecutionContext', 'ShellId', 'StackTrace', 'Input', 'Args', 'This',
    'Sender', 'PSItem', 'Matches', 'ForEach', 'Switch', 'IsWindows',
    'IsLinux', 'IsMacOS', 'PSStyle', 'Error', 'LastExitCode',
    'NestedPromptLevel', 'PSNativeCommandArgumentPassing',
    'PSSessionApplicationName', 'MyExecutionContext', 'IsCoreCLR'
)

$readOnly = [System.Collections.Generic.List[string]]::new()
foreach ($name in $candidates) {
    # [FACT] The assignment has to be a literal one in the generated text:
    # `[scriptblock]::Create` compiles the string, so the name has to be
    # pasted into it rather than interpolated at run time.
    $code = "`$ErrorActionPreference='Stop'; try { `$$name = 'x'; 'WRITABLE' } catch { 'READONLY' }"
    $result = & ([scriptblock]::Create($code))
    if ($result -eq 'READONLY') { $readOnly.Add($name) }
}

Write-Host ("    the read-only automatic variables here: {0}" -f ($readOnly -join ', '))

# [FACT] A measurement that finds nothing is a broken measurement, not a
# clean bill of health. If the set is empty, the check would pass
# everything, and it would do so silently.
if ($readOnly.Count -eq 0) {
    Write-Error 'the measurement found no read-only automatic variables at all, so it cannot be trusted'
    exit 2
}

# ── Scan every PowerShell script in the repository ──────────────────────────
$pattern = '(?im)^\s*[$]({0})\s*(=|\+=|-=|\*=|/=)' -f (($readOnly | ForEach-Object { [regex]::Escape($_) }) -join '|')

# [FACT] The exclusions are matched against the path **relative to the
# repository root**, not against the absolute path. An absolute match is
# wrong the moment the repository is checked out under a directory whose
# name happens to look like an exclusion — a worktree called `.worktrees`
# excludes every file in it, and the check then reports "0 scripts, all
# clean", which is the most convincing false green there is.
$files = @(
    Get-ChildItem -Path $RepoRoot -Recurse -File -Filter '*.ps1' -ErrorAction SilentlyContinue |
        Where-Object {
            $rel = $_.FullName.Substring($RepoRoot.Length).TrimStart([char]'\', [char]'/')
            $rel -notlike 'node_modules/*' -and
            $rel -notlike 'target/*' -and
            $rel -notlike 'target-*/*' -and
            $rel -notlike '.git/*' -and
            $rel -notlike '.worktrees/*'
        } |
        Sort-Object FullName
)

# [FACT] A scan that finds no scripts at all is a broken scan, not a
# clean bill of health — the repository has PowerShell scripts, and a
# check that silently scans nothing passes everything.
if ($files.Count -eq 0) {
    Write-Error "no PowerShell scripts were found under $RepoRoot, so nothing was checked"
    exit 2
}

$problems = @()
foreach ($file in $files) {
    $relative = $file.FullName.Substring($RepoRoot.Length).TrimStart([char]'\', [char]'/')
    $lines = $null
    try {
        $lines = Get-Content -LiteralPath $file.FullName -ErrorAction Stop
    } catch {
        $problems += "${relative}: the file could not be read, so it could not be checked: $_"
        continue
    }
    for ($i = 0; $i -lt $lines.Count; $i++) {
        if ($lines[$i] -match $pattern) {
            $problems += ("{0}:{1}: {2}" -f $relative, ($i + 1), $lines[$i].Trim())
        }
    }
}

if ($problems.Count -gt 0) {
    Write-Host ''
    foreach ($problem in $problems) {
        Write-Host "    $problem" -ForegroundColor Red
    }
    Write-Host ''
    Write-Error ("{0} PowerShell script(s) assign to a read-only automatic variable." -f $problems.Count)
    Write-Host '  Assigning to one of those throws, and a script running under' -ForegroundColor Yellow
    Write-Host "  `$ErrorActionPreference = 'Stop' aborts at the assignment — leaving any child" -ForegroundColor Yellow
    Write-Host '  process it had already started running. Rename the variable.' -ForegroundColor Yellow
    exit 1
}

Write-Host ("    ok  {0} PowerShell script(s), none of them assigns to a read-only automatic variable" -f $files.Count)
exit 0
