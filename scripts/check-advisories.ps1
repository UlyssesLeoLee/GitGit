#requires -Version 7
<#
.SYNOPSIS
    Advisory ratchet gate — compares what `cargo audit --json` reports for a
    lockfile against the committed baseline in scripts/advisory-baseline.json.

.DESCRIPTION
    Why this exists
    ---------------
    Plain `cargo audit` exits 0 when the only findings are `unmaintained`,
    `unsound` or `yanked`. It fails on *vulnerabilities* and nothing else. So
    a newly disclosed advisory that lands in one of those three classes passes
    CI without a word. This repository has already paid for that shape once:
    the unit-test count contract (scripts/regression-baseline.json) sat
    unenforced for 19 days and 28 tests landed unnoticed.

    This gate is ADDITIVE. Plain `cargo audit` keeps running, unmodified, and
    keeps failing on vulnerabilities. This script never weakens it: there is no
    `--ignore`, no `audit.toml` ignore list, and no `-D warnings`. It adds a
    second, stricter question:

        "is the set of advisories cargo audit reports still EXACTLY the set the
         baseline records?"

    Contract
    --------
      * A finding that cargo audit reports but the baseline does not record is
        a FAILURE, and it is named. This is the ratchet: a newly disclosed
        advisory turns the gate red until somebody deliberately re-agrees the
        baseline in a reviewed diff.
      * A baseline entry that no longer applies is GOOD NEWS and must never
        fail the build. It is reported as information ("N baseline entries no
        longer apply — update the baseline") and the exit code stays 0.
      * A baseline entry that names an advisory ID which does not exist in the
        advisory database is a FAILURE, and it is named. Without this check a
        fabricated entry would sit in the baseline forever, doing nothing, and
        would look like a deliberate, reviewed decision. A typo'd ID and a
        made-up one are indistinguishable to a set comparison, so the baseline
        is validated against the advisory database itself.

    Mutation-proofing
    ------------------
    The failure modes are proven by mutation, not asserted. See the lane report.

    Exit codes
    ----------
      0  no new advisories, no fabricated baseline entries
      1  a failure condition was found (new advisory, version drift, or a
         fabricated baseline entry)
      2  setup error (cargo-audit missing, lockfile missing, baseline missing
         or malformed, advisory database unreachable, cargo audit produced no
         parsable JSON)

.PARAMETER Scope
    Which lockfile to check: `root` (Cargo.lock), `tauri`
    (apps/gm-desktop/src-tauri/Cargo.lock), or `all`.

.PARAMETER DbPath
    Advisory database directory. When omitted it is resolved the way
    cargo-audit resolves it, and the SAME resolved path is handed to
    cargo-audit via `--db`, so the database the baseline is validated against
    is exactly the database the scan used.

.PARAMETER BaselinePath
    Override for the baseline file. Defaults to
    <repo>/scripts/advisory-baseline.json.

.EXAMPLE
    pwsh -NoProfile -File scripts/check-advisories.ps1 -Scope root

.EXAMPLE
    pwsh -NoProfile -File scripts/check-advisories.ps1 -Scope all
#>

[CmdletBinding()]
param(
    [ValidateSet('root', 'tauri', 'all')]
    [string]$Scope = 'all',

    [string]$DbPath,

    [string]$BaselinePath
)

$ErrorActionPreference = 'Stop'

# Reuse the house primitives: Get-RepoRoot, Get-ToolPath, Invoke-External and
# the colour-agnostic [step]/[pass]/[fail]/[warn]/[info] printers. The exit
# code contract (0 = pass, 1 = fail, 2 = setup error) matches the regression
# tiers' convention.
. (Join-Path $PSScriptRoot 'lib/regression-common.ps1')

# ── Baseline ────────────────────────────────────────────────────────────────

$repoRoot = Get-RepoRoot
if (-not $BaselinePath) {
    $BaselinePath = Join-Path $repoRoot 'scripts/advisory-baseline.json'
}
if (-not (Test-Path -LiteralPath $BaselinePath)) {
    Write-Fail "baseline not found at $BaselinePath"
    exit 2
}
try {
    $baseline = Get-Content -LiteralPath $BaselinePath -Raw -Encoding utf8 | ConvertFrom-Json
} catch {
    Write-Fail "baseline at $BaselinePath is not valid JSON: $($_.Exception.Message)"
    exit 2
}
if (-not $baseline.scopes) {
    Write-Fail "baseline at $BaselinePath has no 'scopes' object"
    exit 2
}

$scopes = if ($Scope -eq 'all') { @('root', 'tauri') } else { @($Scope) }

# ── Advisory database resolution ─────────────────────────────────────────────

function Resolve-AdvisoryDb {
    <#
      Resolve the advisory database directory the way cargo-audit does.

      Order: explicit parameter, then $CARGO_AUDIT_DB, then
      $CARGO_HOME/advisory-db, then ~/.cargo/advisory-db. The result is
      handed to cargo-audit via `--db` so that the scan and the baseline
      validation read the SAME database — validating against a different
      database than the one that was scanned would be a check of nothing.
    #>
    if ($DbPath) { return $DbPath }
    if ($env:CARGO_AUDIT_DB) { return $env:CARGO_AUDIT_DB }
    if ($env:CARGO_HOME) { return (Join-Path $env:CARGO_HOME 'advisory-db') }
    $homeDir = if ($env:HOME) { $env:HOME } else { $env:USERPROFILE }
    if ($homeDir) { return (Join-Path $homeDir '.cargo/advisory-db') }
    return $null
}

function Get-KnownAdvisoryIds {
    <#
      Every advisory ID that exists in the local advisory database.

      The rustsec/advisory-db layout is `crates/<package>/<ID>.md` plus
      `rust/<vendor>/<ID>.md`. Measured on the database this repository scans:
      1332 + 20 = 1352 files, which is exactly the "Loaded 1352 security
      advisories" count cargo-audit prints — so this enumeration sees the
      whole database and not a subset of it.

      Returns $null when the database cannot be read, which the caller treats
      as a setup error rather than as "no advisories exist" — an empty set
      would make every baseline entry look fabricated.
    #>
    $ids = New-Object 'System.Collections.Generic.HashSet[string]'
    $patterns = @(
        (Join-Path $db 'crates/*/*.md'),
        (Join-Path $db 'rust/*/*.md')
    )
    $found = 0
    foreach ($pattern in $patterns) {
        $files = @(Get-ChildItem -Path $pattern -File -ErrorAction SilentlyContinue)
        foreach ($f in $files) {
            [void]$ids.Add([System.IO.Path]::GetFileNameWithoutExtension($f.Name))
            $found++
        }
    }
    if ($found -eq 0) { return $null }
    return $ids
}

function Get-LockPackages {
    <#
      Every `name@version` pair in a Cargo.lock, so the checker can tell a
      baseline entry that names a package the lock does not contain (which is
      only possible for a `yanked` entry — a yanked crate that is in the lock
      is always reported by cargo audit) from one that does.
    #>
    $set = New-Object 'System.Collections.Generic.HashSet[string]'
    $text = Get-Content -LiteralPath $lockPath -Raw -Encoding utf8
    foreach ($block in ($text -split '(?m)^\[\[package\]\]\s*$')) {
        $name = [regex]::Match($block, '(?m)^name\s*=\s*"([^"]+)"')
        $version = [regex]::Match($block, '(?m)^version\s*=\s*"([^"]+)"')
        if ($name.Success -and $version.Success) {
            [void]$set.Add("$($name.Groups[1].Value)@$($version.Groups[1].Value)")
        }
    }
    return $set
}

function Get-ReportedFindings {
    <#
      Normalise `cargo audit --json` output into a flat list of findings.

      Two sources, and the second is the one plain cargo audit does not fail
      on:
        * `vulnerabilities.list` — kind `vulnerability`
        * `warnings.<kind>[]`   — kind `unmaintained` / `unsound` / `yanked`
          (a `yanked` finding carries no advisory object, so its Id is $null)

      The JSON is PARSED, never grepped. A substring match on the raw output
      can be satisfied by an advisory ID appearing in a description, a URL or a
      comment; this repository has already been bitten by that shape.
    #>
    $findings = New-Object System.Collections.Generic.List[object]
    foreach ($v in @($auditJson.vulnerabilities.list)) {
        if ($null -eq $v) { continue }
        $findings.Add([pscustomobject]@{
            Kind    = 'vulnerability'
            Id      = if ($v.advisory) { [string]$v.advisory.id } else { $null }
            Package = [string]$v.package.name
            Version = [string]$v.package.version
        })
    }
    $warnings = $auditJson.warnings
    if ($warnings) {
        foreach ($prop in $warnings.PSObject.Properties) {
            foreach ($w in @($prop.Value)) {
                if ($null -eq $w) { continue }
                $findings.Add([pscustomobject]@{
                    Kind    = [string]$prop.Name
                    Id      = if ($w.advisory) { [string]$w.advisory.id } else { $null }
                    Package = [string]$w.package.name
                    Version = [string]$w.package.version
                })
            }
        }
    }
    return $findings
}

# ── Advisory database ───────────────────────────────────────────────────────

$db = Resolve-AdvisoryDb
if (-not $db) {
    Write-Fail 'cannot resolve the advisory database path (no -DbPath, no CARGO_AUDIT_DB, no CARGO_HOME, no HOME)'
    exit 2
}
Write-Step "advisory database: $db"

# ── cargo / cargo-audit binaries ────────────────────────────────────────────

# The scan is driven through `cargo audit`, not through `cargo-audit` directly.
# cargo-audit is a cargo SUBCOMMAND: running the `cargo-audit` binary on its
# own makes it hand the arguments to `cargo`, which rejects `--json` with
# "unexpected argument '--json' found" and exits 2. `cargo audit ...` is the
# spelling the CI workflows already use.
$cargoBin = Get-ToolPath -Name 'cargo'
if (-not $cargoBin) {
    Write-Fail 'cargo not found on PATH'
    exit 2
}
if (-not (Get-ToolPath -Name 'cargo-audit')) {
    Write-Fail 'cargo-audit not found on PATH (install it with: cargo install cargo-audit --locked --version 0.22.2)'
    exit 2
}

$knownIds = Get-KnownAdvisoryIds
if ($null -eq $knownIds) {
    Write-Fail "advisory database at '$db' could not be read — the baseline cannot be validated, so this gate checks nothing"
    exit 2
}
Write-Step "advisory database holds $($knownIds.Count) advisory IDs"

# ── Per-scope check ─────────────────────────────────────────────────────────

$overallFail = 0

foreach ($scopeName in $scopes) {
    $scopeDef = $baseline.scopes.$scopeName
    if ($null -eq $scopeDef) {
        Write-Fail "the baseline has no scope '$scopeName'"
        $overallFail = 1
        continue
    }

    $lockRel = [string]$scopeDef.lockfile
    $lockPath = Join-Path $repoRoot ($lockRel -replace '/', [System.IO.Path]::DirectorySeparatorChar)
    if (-not (Test-Path -LiteralPath $lockPath)) {
        Write-Fail "lockfile not found: $lockPath"
        exit 2
    }

    Write-Step "scope '$scopeName': scanning $lockRel"

    # `--db` is passed explicitly so the scan and the baseline validation read
    # the same database. --json is parsed below; the plain-text gate stays a
    # separate, unmodified `cargo audit` run in CI.
    $audit = Invoke-External -FilePath $cargoBin -ArgumentList @(
        'audit',
        '--json',
        '--db', $db,
        '--file', $lockPath
    )

    if ([string]::IsNullOrWhiteSpace($audit.StdOut)) {
        Write-Fail "cargo audit produced no output for $lockRel (exit $($audit.ExitCode))"
        if ($audit.StdErr) { Write-Host $audit.StdErr }
        exit 2
    }
    try {
        $auditJson = $audit.StdOut | ConvertFrom-Json
    } catch {
        Write-Fail "cargo audit output for $lockRel is not parsable JSON (exit $($audit.ExitCode)): $($_.Exception.Message)"
        exit 2
    }

    # Fail closed when the advisory database did not load. An empty advisory
    # set would make every baseline entry look fabricated and, worse, would
    # hide a newly disclosed advisory behind a scan that checked nothing.
    $dbCount = 0
    if ($auditJson.database -and $auditJson.database.'advisory-count') {
        $dbCount = [int]$auditJson.database.'advisory-count'
    }
    if ($dbCount -le 0) {
        Write-Fail "cargo audit reported 0 advisories in the database — the scan checked nothing"
        exit 2
    }

    $reported = Get-ReportedFindings
    $entries = @($scopeDef.advisories)

    # -- Baseline lookup --------------------------------------------------
    # Identity is (kind, id, package). A `yanked` finding has no advisory ID,
    # so its identity is (kind, package) alone.
    $baselineByIdentity = @{}
    foreach ($e in $entries) {
        $key = "$($e.kind)|$($e.id)|$($e.package)"
        $baselineByIdentity[$key] = $e
    }

    $newFindings = New-Object System.Collections.Generic.List[object]
    $drifted = New-Object System.Collections.Generic.List[object]
    $matched = New-Object System.Collections.Generic.HashSet[string]

    foreach ($f in $reported) {
        $key = "$($f.Kind)|$($f.Id)|$($f.Package)"
        if (-not $baselineByIdentity.ContainsKey($key)) {
            $newFindings.Add($f)
            continue
        }
        [void]$matched.Add($key)
        $b = $baselineByIdentity[$key]
        if ([string]$b.version -ne [string]$f.Version) {
            $drifted.Add([pscustomobject]@{
                Kind    = $f.Kind
                Id      = $f.Id
                Package = $f.Package
                Was     = [string]$b.version
                Now     = [string]$f.Version
            })
        }
    }

    $stale = New-Object System.Collections.Generic.List[object]
    foreach ($e in $entries) {
        $key = "$($e.kind)|$($e.id)|$($e.package)"
        if (-not $matched.Contains($key)) { $stale.Add($e) }
    }

    # -- Fabrication check -------------------------------------------------
    # A baseline entry that names an advisory ID which is not in the advisory
    # database cannot have come from a scan; it is a typo or an invention.
    # A `yanked` entry has no ID, so it is validated against the lockfile
    # instead: a yanked crate that is in the lock is always reported.
    $lockPackages = Get-LockPackages
    $fabricated = New-Object System.Collections.Generic.List[object]
    foreach ($e in $entries) {
        if ($e.id) {
            if (-not $knownIds.Contains([string]$e.id)) {
                $fabricated.Add([pscustomobject]@{
                    Reason  = 'advisory ID is not in the advisory database'
                    Id      = [string]$e.id
                    Package = [string]$e.package
                    Version = [string]$e.version
                })
            }
        } else {
            if (-not $lockPackages.Contains("$($e.package)@$($e.version)")) {
                $fabricated.Add([pscustomobject]@{
                    Reason  = 'names a package@version that is not in the lockfile'
                    Id      = '(yanked — no advisory ID)'
                    Package = [string]$e.package
                    Version = [string]$e.version
                })
            }
        }
    }

    # -- Report ------------------------------------------------------------
    $scopeFailed = $false

    foreach ($f in $newFindings) {
        $idText = if ($f.Id) { $f.Id } else { '(no advisory ID — yanked)' }
        Write-Fail "NEW ADVISORY: $idText  $($f.Package) $($f.Version)  [$($f.Kind)] is reported by cargo audit but is not in the baseline"
        $scopeFailed = $true
    }
    foreach ($d in $drifted) {
        $idText = if ($d.Id) { $d.Id } else { '(no advisory ID — yanked)' }
        Write-Fail "VERSION DRIFT: $idText  $($d.Package) — the baseline records $($d.Was) but cargo audit now reports $($d.Now); re-agree the baseline"
        $scopeFailed = $true
    }
    foreach ($f in $fabricated) {
        Write-Fail "FABRICATED BASELINE ENTRY: $($f.Id)  $($f.Package) $($f.Version) — $($f.Reason)"
        $scopeFailed = $true
    }

    if ($stale.Count -gt 0) {
        Write-Info "$($stale.Count) baseline entries no longer apply — update the baseline:"
        foreach ($e in $stale) {
            $idText = if ($e.id) { $e.id } else { '(no advisory ID — yanked)' }
            Write-Host "         $idText  $($e.package) $($e.version)  [$($e.kind)]"
        }
    }

    if ($scopeFailed) {
        Write-Fail "scope '$scopeName': FAILED — $($newFindings.Count) new, $($drifted.Count) drifted, $($fabricated.Count) fabricated, $($stale.Count) stale"
        $overallFail = 1
    } else {
        Write-Pass "scope '$scopeName': ok — $($reported.Count) advisories reported, all in the baseline; $($stale.Count) stale"
    }
}

if ($overallFail -ne 0) {
    Write-Fail 'advisory ratchet: FAILED'
    exit 1
}

Write-Pass 'advisory ratchet: ok'
exit 0
