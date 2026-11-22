#requires -Version 7
<#
.SYNOPSIS
    Refuse a declared `rust-version` that the locked dependency tree cannot honour.

.DESCRIPTION
    `rust-version` in a manifest is a promise: "this crate builds with this
    toolchain". Cargo uses it to tell a user whose compiler is too old, before
    the build fails somewhere deep inside a dependency.

    [FACT] That promise is only as good as the lockfile under it. The root
    crate's dependencies ask for Rust 1.88 (time 0.3.55, sysinfo 0.37.2, the
    icu_* 2.x crates) and the desktop crate's own lockfile asks for 1.90
    (tauri 2.12.3, tauri-utils 2.10.2, tray-icon 0.25.1). Both manifests
    used to declare 1.75, which was never true of either tree.

    [FACT] The floor is **measured** on every run, not copied into this
    script: `cargo metadata --locked` reports the `rust_version` each locked
    dependency declares, and the highest of them is the tree's floor. A copied
    number is a number that drifts the moment a dependency is updated; a
    measurement is not.

    [FACT] The check runs `cargo metadata`, which resolves no dependency
    graph of its own and builds nothing — it reads the lockfile and the
    registry index. That is why it belongs in the always-running gate: it
    costs seconds, it is true of every pull request, and it can go red only
    because of the pull request's own content (a manifest or a lockfile) or
    because a dependency raised its floor — and the second case is exactly
    the case this check exists to catch.

.PARAMETER RepoRoot
    The repository to check. Default: the repository this script lives in.

.EXAMPLE
    pwsh -NoProfile -File scripts/ci/check-msrv.ps1
#>

[CmdletBinding()]
param(
    [string]$RepoRoot = ''
)

$ErrorActionPreference = 'Stop'

if (-not $RepoRoot) {
    $RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
}

# The two crates, each with its own manifest and its own lockfile.
$crates = @(
    @{ Name = 'the root crate'; Manifest = 'Cargo.toml' },
    @{ Name = 'the desktop crate'; Manifest = 'apps/gm-desktop/src-tauri/Cargo.toml' }
)

$failures = 0

function Get-Version([string]$value) {
    # "1.89" -> [version]; a leading "v" is tolerated.
    $text = $value.Trim().TrimStart('v')
    try {
        return [version]$text
    } catch {
        return $null
    }
}

foreach ($crate in $crates) {
    $manifestPath = Join-Path $RepoRoot $crate.Manifest
    if (-not (Test-Path -LiteralPath $manifestPath)) {
        Write-Error "the manifest is missing: $manifestPath"
        exit 2
    }

    # The declared version, read from the manifest's [package] table.
    $declared = $null
    $inPackage = $false
    foreach ($line in Get-Content -LiteralPath $manifestPath) {
        if ($line -match '^\s*\[package\]\s*$') { $inPackage = $true; continue }
        if ($line -match '^\s*\[') { $inPackage = $false }
        if (-not $inPackage) { continue }
        if ($line -match '^\s*rust-version\s*=\s*"([^"]+)"') {
            $declared = Get-Version $Matches[1]
            break
        }
    }
    if (-not $declared) {
        Write-Error "$($crate.Manifest) declares no readable rust-version"
        exit 2
    }

    # The measured floor: the highest rust_version among the locked
    # dependencies. `cargo metadata` resolves nothing and builds nothing.
    $raw = & cargo metadata --locked --format-version 1 --manifest-path $manifestPath 2>$null
    if ($LASTEXITCODE -ne 0 -or -not $raw) {
        Write-Error "cargo metadata failed for $($crate.Manifest)"
        exit 2
    }
    # [FACT] `-AsHashtable`, because the metadata document carries keys that
    # differ only in case (the `usb` / `USB` pair among the target
    # triples), and `ConvertFrom-Json` refuses a document that does.
    $metadata = ($raw -join "`n") | ConvertFrom-Json -AsHashtable

    $floor = $null
    $floorBy = ''
    foreach ($package in $metadata['packages']) {
        # [FACT] Only the REGISTRY dependencies count. A package with no
        # `source` is a local path package — this crate itself, or a
        # workspace member — and counting it would make the check measure
        # its own declaration and always pass.
        if (-not $package['source']) { continue }
        if (-not $package['rust_version']) { continue }
        $version = Get-Version $package['rust_version']
        if (-not $version) { continue }
        if (-not $floor -or $version -gt $floor) {
            $floor = $version
            $floorBy = "$($package['name']) $($package['version'])"
        }
    }
    if (-not $floor) {
        Write-Error "no dependency declares a rust-version for $($crate.Manifest)"
        exit 2
    }

    if ($declared -lt $floor) {
        Write-Output "[fail] $($crate.Name) declares rust-version $declared, but its locked dependencies need $floor (highest: $floorBy)"
        $failures += 1
    } else {
        Write-Output "[pass] $($crate.Name) declares rust-version $declared; the locked tree needs $floor (highest: $floorBy)"
    }
}

if ($failures -gt 0) {
    Write-Output "[fail] $failures crate(s) declare an MSRV their locked dependency tree cannot honour"
    exit 1
}
Write-Output '[pass] every declared MSRV is at or above its locked tree''s floor'
exit 0
