#requires -Version 7
<#
.SYNOPSIS
    Frontend dependency-advisory gate — a ratchet over the npm advisory database.

.DESCRIPTION
    Runs `pnpm audit --json` for every frontend named in the baseline, PARSES the
    JSON, and fails on any advisory that is not recorded in the baseline.

    Why a ratchet and not a hard gate:
    At capture time the frontends carry 17 open advisories. A hard gate (fail on
    anything present) would have been red on its very first run and would have
    been switched off — which is what happened to the Rust-side unit-test count
    contract, left unenforced for 19 days. This gate instead fails only on a
    NEW advisory, which is the property actually worth protecting: a newly
    disclosed npm advisory must not land silently.

    What the baseline is, and is not:
    It is a RECORD of what is known. Every entry carries the GHSA id, the
    module, the exact version that was found and the patched range, and the
    checker re-verifies each entry against the npm advisory database on every
    run. An entry the database cannot corroborate is rejected and fails the
    build. That is what stops the baseline from being used as an ignore list:
    there is no 'ignore' or 'waiver' section, `pnpm audit --ignore` is never
    used, and no `pnpm.overrides` entry pins a vulnerable version.

    The three verdicts:
      * new        — in the audit, not in the baseline. FAILS, and names it.
      * resolved   — in the baseline, no longer in the audit. Informational.
                     An advisory that disappeared is good news, not a failure.
      * fabricated — in the baseline, but the npm advisory database does not
                     report it for the recorded module and version. FAILS, and
                     names it. This is the check that separates "we fixed it"
                     from "somebody typed an ID in by hand".

    Exit codes (the house contract, from scripts/lib/regression-common.ps1):
      0 = no new and no fabricated advisories
      1 = at least one new or fabricated advisory
      2 = setup error (pnpm missing, audit output unparseable, registry
          unreachable). A gate that quietly passed because it checked nothing
          would be worse than no gate, so this is a failure, not a pass.

.PARAMETER BaselinePath
    Path to the baseline JSON. Defaults to scripts/frontend-advisory-baseline.json.

.PARAMETER PnpmVersion
    The pnpm version to run. Pinned deliberately: a gate whose verdicts drift
    with the tool's release is not a gate.

.EXAMPLE
    pwsh -NoProfile -File scripts/frontend-advisory-check.ps1

.NOTES
    [FACT] The JSON is parsed with ConvertFrom-Json. This gate deliberately does
    not grep the audit's text: a gate that can be satisfied by an advisory ID
    appearing in a comment is not a gate, and this repository has been bitten by
    exactly that.
#>

param(
    [string]$BaselinePath,
    [string]$PnpmVersion = '9.15.9'
)

$ErrorActionPreference = 'Stop'
. $PSScriptRoot\lib\regression-common.ps1

$repoRoot = Get-RepoRoot
if (-not $BaselinePath) {
    $BaselinePath = Join-Path $repoRoot 'scripts/frontend-advisory-baseline.json'
}
if (-not (Test-Path -LiteralPath $BaselinePath)) {
    Write-Fail "baseline not found: $BaselinePath"
    exit 2
}

try {
    $baseline = Get-Content -Raw -LiteralPath $BaselinePath | ConvertFrom-Json
} catch {
    Write-Fail "baseline is not valid JSON: $($_.Exception.Message)"
    exit 2
}
if (-not $baseline.apps) {
    Write-Fail 'baseline has no `apps` object'
    exit 2
}

$corepack = Get-ToolPath -Name 'corepack'
$pathPnpm = Get-ToolPath -Name 'pnpm'
$pnpmExe = $null
$pnpmArgs = @()

# Resolve how to run pnpm, at exactly $PnpmVersion.
#
# The version is pinned because a gate whose verdicts drift with the tool's
# release is not a gate — and because the bare `pnpm` on a developer machine is
# frequently a different major than the one the baseline was captured with
# (measured on this box: `pnpm` on PATH reports 10.32.1 while the baseline
# was captured with 9.15.9, and `pnpm audit --json`'s output format is not
# guaranteed to be identical across majors).
#
# Two ways to get exactly that version:
#   * a `pnpm` already on PATH that reports it — CI installs exactly this
#     version with pnpm/action-setup, so the runner never depends on
#     corepack's download-and-verify path; or
#   * `corepack pnpm@<version>`, which fetches that exact version.
$pathPnpmVersion = $null
if ($pathPnpm) {
    $probe = Invoke-External -FilePath $pathPnpm -ArgumentList @('--version') -TimeoutSec 120
    if ($probe.ExitCode -eq 0) { $pathPnpmVersion = $probe.StdOut.Trim() }
}
if ($pathPnpmVersion -eq $PnpmVersion) {
    $pnpmExe = $pathPnpm
    $pnpmArgs = @()
} elseif ($corepack) {
    $pnpmExe = $corepack
    $pnpmArgs = @("pnpm@$PnpmVersion")
    if ($pathPnpmVersion) {
        Write-Info "pnpm on PATH reports '$pathPnpmVersion', not $PnpmVersion; using corepack pnpm@$PnpmVersion"
    }
} else {
    Write-Fail "no pnpm $PnpmVersion on PATH and no corepack to fetch it with; cannot run pnpm audit"
    exit 2
}

$run = New-RegressionRun -Tier 'frontend-audit'
Write-Step "frontend advisory ratchet — baseline: $BaselinePath"

# ── Helpers ──────────────────────────────────────────────────────────────────

function Get-GhsaId {
    # pnpm's audit JSON carries `github_advisory_id` directly; older shapes
    # only expose the id inside the `references` list, so fall back to that.
    param([psobject]$Advisory)
    if ($Advisory.github_advisory_id) { return [string]$Advisory.github_advisory_id }
    $blob = ($Advisory.references -join "`n")
    $m = [regex]::Match($blob, 'GHSA-[0-9a-z-]+')
    if ($m.Success) { return $m.Value }
    return $null
}

function Get-EntryKey {
    # The same GHSA id can cover two packages (GHSA-82fw-gwwq-j7x9 is reported
    # against both `vitest` and `@vitest/mocker`), so the identity of an
    # advisory here is the (module, id) pair, not the id alone.
    param([string]$Module, [string]$Id)
    return "$Module@$Id"
}

function Get-EntryVersions {
    # `version` is a single version in the current baseline but is written as
    # a comma-separated list to survive a package appearing at two versions.
    param([psobject]$Entry)
    return @([string]$Entry.version -split ',' | ForEach-Object { $_.Trim() } | Where-Object { $_ })
}

function Invoke-AuditJson {
    <#
      Runs `pnpm audit --json` for one app and returns the parsed document.

      `pnpm audit` exits 1 when advisories are present. That is the expected
      outcome for a tree that has known advisories, so it is not an error
      here; only an exit code outside {0,1}, or output that does not parse,
      is a setup error.
    #>
    param([string]$App)

    $appDir = Join-Path $repoRoot "apps/$App"
    if (-not (Test-Path -LiteralPath $appDir)) {
        throw "app directory not found: $appDir"
    }
    $auditArgs = @($pnpmArgs + @('--dir', $appDir, 'audit', '--json'))
    $result = Invoke-External -FilePath $pnpmExe `
        -ArgumentList $auditArgs `
        -WorkingDirectory $repoRoot -TimeoutSec 600

    if ($result.ExitCode -ne 0 -and $result.ExitCode -ne 1) {
        throw "pnpm audit exited $($result.ExitCode) for $App (expected 0 or 1)`n$($result.StdErr)"
    }
    $text = $result.StdOut
    if ([string]::IsNullOrWhiteSpace($text)) {
        throw "pnpm audit produced no output for $App`n$($result.StdErr)"
    }
    $start = $text.IndexOf('{')
    if ($start -lt 0) {
        throw "pnpm audit output for $App contained no JSON object"
    }
    try {
        $doc = $text.Substring($start) | ConvertFrom-Json
    } catch {
        throw "pnpm audit output for $App is not parseable JSON: $($_.Exception.Message)"
    }
    if (-not $doc.advisories) {
        throw "pnpm audit output for $App has no 'advisories' object"
    }
    return $doc
}

function Get-RegistryAdvisoryIds {
    <#
      Asks the npm registry which advisories it holds for a set of
      module@version pairs. Returns a hashtable: module -> list of GHSA ids.

      The npm registry is the only external service this gate uses. It is the
      same database `pnpm audit` itself queries, so a baseline entry that it
      cannot corroborate is not a real advisory.
    #>
    param([hashtable]$Wanted)

    $body = @{}
    foreach ($module in $Wanted.Keys) {
        $body[$module] = @($Wanted[$module])
    }
    $json = $body | ConvertTo-Json -Depth 5 -Compress

    $response = Invoke-RestMethod `
        -Uri 'https://registry.npmjs.org/-/npm/v1/security/advisories/bulk' `
        -Method Post -ContentType 'application/json' -Body $json -TimeoutSec 120

    $result = @{}
    foreach ($module in $Wanted.Keys) {
        $ids = @()
        $list = $response.PSObject.Properties[$module]
        if ($list -and $list.Value) {
            foreach ($adv in @($list.Value)) {
                if ($adv.url) {
                    $m = [regex]::Match([string]$adv.url, 'GHSA-[0-9a-z-]+')
                    if ($m.Success) { $ids += $m.Value }
                }
            }
        }
        $result[$module] = $ids
    }
    return $result
}

# ── Per-app checks ───────────────────────────────────────────────────────────

foreach ($appProperty in $baseline.apps.PSObject.Properties) {
    $app = $appProperty.Name
    $appBaseline = $appProperty.Value
    Write-Step "app: $app"

    # 1. Measure.
    try {
        $doc = Invoke-AuditJson -App $app
    } catch {
        Write-Fail "audit could not be completed for $app : $($_.Exception.Message)"
        $run.SetupError = $true
        continue
    }

    # 2. Index what the audit found, by (module, GHSA id).
    $found = @{}
    foreach ($property in $doc.advisories.PSObject.Properties) {
        $advisory = $property.Value
        $ghsa = Get-GhsaId -Advisory $advisory
        if (-not $ghsa) {
            Write-Warn "advisory $($advisory.id) has no GHSA id; it cannot be matched against the baseline"
            continue
        }
        $found[(Get-EntryKey -Module $advisory.module_name -Id $ghsa)] = [pscustomobject]@{
            Id       = $ghsa
            Module   = $advisory.module_name
            Severity = $advisory.severity
            Version  = ($advisory.findings | Select-Object -First 1).version
            Patched  = $advisory.patched_versions
            Title    = $advisory.title
        }
    }

    # 3. Index the baseline for this app.
    $open = @{}
    foreach ($entry in @($appBaseline.open)) {
        $open[(Get-EntryKey -Module $entry.module -Id $entry.id)] = $entry
    }
    $resolvedRecorded = @($appBaseline.resolved)

    # 4. NEW — in the audit, not in the baseline. This is the ratchet.
    foreach ($key in ($found.Keys | Sort-Object)) {
        if (-not $open.ContainsKey($key)) {
            $f = $found[$key]
            Assert-True -Run $run `
                -Name "$app : new advisory $($f.Id) ($($f.Module))" `
                -Condition $false `
                -Detail ("not in the baseline; {0} {1}, severity {2}, patched in {3}. Fix it, then record it in {4}" -f `
                    $f.Module, $f.Version, $f.Severity, $f.Patched, (Split-Path -Leaf $BaselinePath))
        } else {
            Assert-True -Run $run `
                -Name "$app : known advisory $($found[$key].Id) ($($found[$key].Module))" `
                -Condition $true `
                -Detail "recorded in the baseline"
        }
    }

    # 5. RESOLVED — in the baseline, no longer in the audit. Informational.
    foreach ($key in ($open.Keys | Sort-Object)) {
        if (-not $found.ContainsKey($key)) {
            $entry = $open[$key]
            Write-Info "$app : resolved advisory $($entry.Id) ($($entry.Module)) is no longer reported — move it from 'open' to 'resolved' in the baseline when you next edit it"
        }
    }

    # 6. FABRICATED — in the baseline, but not corroborated by the npm
    #    advisory database for the recorded module and version.
    $wanted = @{}
    foreach ($entry in (@($appBaseline.open) + $resolvedRecorded)) {
        if (-not $wanted.ContainsKey($entry.module)) { $wanted[$entry.module] = @() }
        $wanted[$entry.module] += (Get-EntryVersions -Entry $entry)
    }
    try {
        $registry = Get-RegistryAdvisoryIds -Wanted $wanted
    } catch {
        Write-Fail "the npm advisory database could not be reached, so the baseline could not be corroborated for $app : $($_.Exception.Message)"
        $run.SetupError = $true
        continue
    }
    foreach ($entry in (@($appBaseline.open) + $resolvedRecorded)) {
        $ids = @()
        if ($registry.ContainsKey($entry.module)) { $ids = @($registry[$entry.module]) }
        $versions = (Get-EntryVersions -Entry $entry) -join ', '
        if ($ids -contains $entry.id) {
            Assert-True -Run $run `
                -Name "$app : baseline entry $($entry.Id) ($($entry.module) $versions) is a real advisory" `
                -Condition $true `
                -Detail "corroborated by the npm advisory database"
        } else {
            Assert-True -Run $run `
                -Name "$app : baseline entry $($entry.Id) ($($entry.module) $versions) is not a real advisory" `
                -Condition $false `
                -Detail ("the npm advisory database reports no such advisory for {0}@{1}; the baseline entry is fabricated and must be removed" -f $entry.module, $versions)
        }
    }
}

Write-Step 'summary'
Write-Info ("passed={0} failed={1} skipped={2}" -f $run.Pass, $run.Fail, $run.Skip)

if ($run.Fail -gt 0) {
    Write-Fail "$($run.Fail) check(s) failed"
    exit 1
}
if ($run.SetupError) {
    Write-Fail 'setup error — this gate checked nothing, which is not a pass'
    exit 2
}
Write-Pass 'no new and no fabricated frontend advisories'
exit 0
