#requires -Version 7
<#
.SYNOPSIS
    Build the gm-desktop Windows installers (.msi) and keep BOTH install scopes.

.DESCRIPTION
    One command produces every requested installer variant, and no variant can
    overwrite another.

    WHY THIS SCRIPT EXISTS
    ======================
    The Tauri bundler names an MSI `{productName}_{version}_{arch}_{lang}.msi`.
    That name carries the product, the version, the architecture and the
    language - but NOT the install scope. A per-user and a per-machine build of
    the same version and language therefore land on the SAME path in the SAME
    shared bundle directory, and the second build silently overwrites the
    first. The bundler exit code is 0 in both cases, so the loss is invisible
    unless you look at the directory.

    The obvious workaround - giving the two variants different product names -
    is not available: `productName` is the user-visible installed application
    name and the Start Menu folder name, so the two packages would install
    under different names for no user-visible reason.

    So the fix is to move each artifact out of the shared bundle directory the
    moment it is produced, into a deterministic output directory, under a name
    that DOES carry the scope. That is the whole mechanism, and this script is
    the single place it happens, so it cannot be skipped by forgetting a
    manual step.

    WHAT IT DOES, PER SCOPE
    =======================
      1. Runs `tauri build --bundles msi`, with the per-user scope overlay
         (`src-tauri/tauri.peruser.conf.json`) for the perUser variant. The
         base config already carries the per-machine scope and the language
         list, so the perMachine variant needs no overlay.
      2. Immediately records which .msi files in the bundle directory are
         NEWER than the moment the build started, moves each one to the output
         directory as `{product}_{version}_{arch}_{scope}_{lang}.msi`, and
         asserts the original is gone from the bundle directory. That
         assertion is the collision fix, checked rather than assumed.
      3. Fails loudly, with a non-zero exit code, if the build reported
         success but the expected artifact is missing, is unrecognised, or if
         fewer artifacts appeared than languages requested. "Green but
         produced nothing" has bitten this repo more than once, so a build
         that cannot be accounted for is an error, not a warning.
      4. Records the SHA-256 and byte size of every artifact it produced, both
         on stdout and in `manifest.json` in the output directory.
      5. Asserts that, when more than one variant was requested, the variants
         are byte-distinct. Two variants with the same hash would mean the
         scope never reached the package.

    The final assertion needs no installer and no elevation: it is a hash
    comparison. `verify-msi.ps1` is the deeper check - it extracts each MSI
    with `msiexec /a` and reads the MSI tables.

.PARAMETER Scope
    Which install scope(s) to build. `perMachine` uses the base
    `tauri.conf.json` (Program Files, `wix/main.wxs`). `perUser` layers
    `src-tauri/tauri.peruser.conf.json` (%LOCALAPPDATA%\Programs,
    `wix/main-peruser.wxs`, `InstallScope="perUser"`). `both` builds each in
    turn, moving the first one's artifact out of the way before the second
    build starts. Default: `both`.

.PARAMETER Language
    MSI languages to build. Each requested language must yield exactly one
    .msi. Default: the `bundle.windows.wix.language` list in
    `tauri.conf.json`.

.PARAMETER OutputDir
    Where the scope-qualified .msi files and `manifest.json` are written.
    Default: `<TargetDir>\msi-out`. `src-tauri/target/` is git-ignored
    (apps/gm-desktop/.gitignore line 8), so the default leaves the worktree
    clean.

.PARAMETER TargetDir
    Cargo target directory. The bundle directory is derived from it, because
    the Tauri CLI writes the bundle under the cargo target directory rather
    than under `src-tauri/`. Default: `$env:CARGO_TARGET_DIR` when set,
    otherwise `src-tauri/target`. Set a PRIVATE one on a machine where
    `CARGO_TARGET_DIR` is shared between agents; pointing two agents at one
    target directory serialises their builds and can hand one agent the
    other's artifacts.

.PARAMETER SkipAssertDistinct
    Skip the final check that the produced variants are byte-distinct. Only
    useful when debugging the build itself; leaving it off is the point.

.EXAMPLE
    ./build-msi.ps1
    Builds both scopes in both configured languages, into
    `src-tauri/target/msi-out`.

.EXAMPLE
    ./build-msi.ps1 -Scope perMachine -Language en-US
    Builds only the per-machine English installer. One variant, so the
    distinctness assertion does not apply.

.EXAMPLE
    ./build-msi.ps1 -Scope both -OutputDir C:\artifacts\gm-desktop
    Both scopes into a directory outside the repo, for archiving.

.NOTES
    Exit codes
        0  every requested variant was built, moved, hashed and recorded.
        1  any failure; the reason is printed as `BUILD FAILED: <reason>`.

    stderr and exit codes
        The Tauri CLI and cargo both write progress to stderr. Under
        `$ErrorActionPreference = 'Stop'` that can be promoted into a
        terminating exception and hide the real exit code, so
        `$PSNativeCommandUseErrorActionPreference` is pinned off and every
        external command is invoked through `Invoke-Native`, which gates on
        `$LASTEXITCODE` explicitly.

    Cargo.lock
        `apps/gm-desktop/src-tauri/Cargo.lock` is currently out of date with
        the workspace crate it depends on by path, so cargo re-resolves it on
        a Windows build and the build succeeds while rewriting the file. This
        script does not pass `--locked` (that would fail outright) and does not
        commit the result; it prints a warning naming the file and the change
        so the modification is never silent. See
        docs/reports/2026-10-05-bundle-ci/README.md.
#>

[CmdletBinding()]
param(
    [ValidateSet('perUser', 'perMachine', 'both')]
    [string]$Scope = 'both',

    [string[]]$Language,

    [string]$OutputDir,

    [string]$TargetDir,

    [switch]$SkipAssertDistinct
)

$ErrorActionPreference = 'Stop'
# Keep native stderr from being promoted to a terminating error; Invoke-Native
# gates on the real exit code instead. See .NOTES "stderr and exit codes".
if (Get-Variable -Name PSNativeCommandUseErrorActionPreference -ErrorAction SilentlyContinue) {
    $PSNativeCommandUseErrorActionPreference = $false
}

# ---------------------------------------------------------------------------
# Layout
# ---------------------------------------------------------------------------
# This script is apps/gm-desktop/scripts/build-msi.ps1. Everything is resolved
# from $PSScriptRoot so it works from any working directory, which matters
# because the Tauri CLI resolves `--config` paths against the CURRENT
# directory, not against the tauri directory.
$AppDir      = Split-Path -Parent $PSScriptRoot          # apps/gm-desktop
$RepoRoot    = Split-Path -Parent (Split-Path -Parent $AppDir)
$TauriDir    = Join-Path $AppDir 'src-tauri'
$ConfigFile  = Join-Path $TauriDir 'tauri.conf.json'
$PerUserConf = 'src-tauri/tauri.peruser.conf.json'       # relative to $AppDir

if (-not $TargetDir) {
    if ($env:CARGO_TARGET_DIR) { $TargetDir = $env:CARGO_TARGET_DIR }
    else { $TargetDir = Join-Path $TauriDir 'target' }
}
$TargetDir = [System.IO.Path]::GetFullPath($TargetDir)
if (-not $OutputDir) { $OutputDir = Join-Path $TargetDir 'msi-out' }
$OutputDir = [System.IO.Path]::GetFullPath($OutputDir)
# The Tauri CLI puts bundles under <target>/release/bundle/<type> for a host
# build (no --target), which is the only mode this script uses: a cross build
# would need the target triple here and would change the arch in the filename.
$BundleMsiDir = Join-Path $TargetDir 'release\bundle\msi'

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------
function Write-Step {
    param([string]$Message)
    Write-Host "==> $Message" -ForegroundColor Cyan
}

function Write-Ok {
    param([string]$Message)
    Write-Host "    ok  $Message" -ForegroundColor Green
}

function Invoke-Native {
    <#
        Run an external command, gate on its exit code. $ErrorActionPreference
        is deliberately not allowed to be the gate: a non-zero exit must
        surface as a non-zero exit, never as a swallowed green step.
    #>
    param(
        [Parameter(Mandatory)][string]$File,
        [Parameter(Mandatory)][AllowEmptyCollection()][string[]]$Arguments,
        [Parameter(Mandatory)][string]$What
    )
    Write-Host "    $ $File $($Arguments -join ' ')" -ForegroundColor DarkGray
    & $File @Arguments
    $code = $LASTEXITCODE
    if ($code -ne 0) { throw "$What failed with exit code $code" }
}

function Get-ArtifactRecord {
    param(
        [Parameter(Mandatory)][string]$Path,
        [Parameter(Mandatory)][string]$ScopeName,
        [Parameter(Mandatory)][string]$Lang
    )
    $item = Get-Item -LiteralPath $Path
    [pscustomobject]@{
        scope   = $ScopeName
        language = $Lang
        file    = $item.Name
        path    = $item.FullName
        bytes   = $item.Length
        sha256  = (Get-FileHash -LiteralPath $item.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
    }
}

# ---------------------------------------------------------------------------
# Preconditions and inputs
# ---------------------------------------------------------------------------
if (-not (Test-Path -LiteralPath $ConfigFile)) {
    throw "tauri config not found: $ConfigFile"
}
$config = Get-Content -LiteralPath $ConfigFile -Raw -Encoding UTF8 | ConvertFrom-Json
$productName = $config.productName
$version     = $config.version
if (-not $productName -or -not $version) {
    throw "could not read productName/version from $ConfigFile"
}

if (-not $Language -or $Language.Count -eq 0) {
    $Language = @($config.bundle.windows.wix.language)
    if (-not $Language -or $Language.Count -eq 0) {
        throw "no -Language given and bundle.windows.wix.language is empty in $ConfigFile"
    }
}
$Language = @($Language | ForEach-Object { [string]$_ })

# Host architecture, spelled the way the bundler spells it in the filename
# ({arch} is `x86`, `x64` or `arm64`).
$arch = switch ([System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture) {
    'X64'   { 'x64' }
    'X86'   { 'x86' }
    'Arm64' { 'arm64' }
    default { throw "unsupported host architecture for MSI naming: $([System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture)" }
}

$scopes = switch ($Scope) {
    'both'       { @('perMachine', 'perUser') }
    default      { @($Scope) }
}

# The per-user WiX template is generated. If it is missing, the build would
# fail deep inside candle with a confusing message, so check it up front.
$perUserTemplate = Join-Path $TauriDir 'wix\main-peruser.wxs'
if ($scopes -contains 'perUser' -and -not (Test-Path -LiteralPath $perUserTemplate)) {
    throw "per-user WiX template missing: $perUserTemplate (regenerate with apps/gm-desktop/scripts/render-wix-template.py)"
}

# Package manager: prefer a plain `pnpm` (CI installs it with
# pnpm/action-setup), fall back to corepack for machines where pnpm only
# exists as a corepack shim.
$pnpm = if (Get-Command pnpm -ErrorAction SilentlyContinue) { @('pnpm') }
        else { @('corepack', 'pnpm@9.15.9') }
# Everything after the executable name. Written out rather than sliced inline
# because `$arr[1..0]` on a one-element array yields @($null, 'pnpm').
$pnpmArgs = if ($pnpm.Count -gt 1) { @($pnpm[1..($pnpm.Count - 1)]) } else { @() }

Write-Step "gm-desktop MSI build"
Write-Host "    product     : $productName $version ($arch)"
Write-Host "    scope(s)    : $($scopes -join ', ')"
Write-Host "    language(s) : $($Language -join ', ')"
Write-Host "    bundle dir  : $BundleMsiDir"
Write-Host "    output dir  : $OutputDir"
Write-Host "    pnpm        : $($pnpm -join ' ')"

New-Item -ItemType Directory -Path $OutputDir -Force | Out-Null

$lockFile = Join-Path $TauriDir 'Cargo.lock'
$lockBefore = if (Test-Path -LiteralPath $lockFile) {
    (Get-FileHash -LiteralPath $lockFile -Algorithm SHA256).Hash
} else { $null }

# ---------------------------------------------------------------------------
# Build loop
# ---------------------------------------------------------------------------
$records = @()

foreach ($scopeName in $scopes) {
    Write-Step "build $scopeName"

    $tauriArgs = @('tauri', 'build', '--bundles', 'msi', '--ci')
    if ($scopeName -eq 'perUser') {
        # Layered last, so it wins over the base config's per-machine scope.
        $tauriArgs += @('--config', $PerUserConf)
    }
    if (($Language -join ',') -ne (@($config.bundle.windows.wix.language) -join ',')) {
        # Only override when the request differs from the config, so the
        # common path builds exactly what tauri.conf.json declares.
        $langJson = '[' + (($Language | ForEach-Object { '"' + $_ + '"' }) -join ',') + ']'
        $tauriArgs += @('--config', ('{"bundle":{"windows":{"wix":{"language":' + $langJson + '}}}}'))
    }

    $startedUtc = [DateTime]::UtcNow
    Invoke-Native -File $pnpm[0] -Arguments ($pnpmArgs + $tauriArgs) `
        -What "tauri build ($scopeName)"

    # Everything the bundler wrote after the build started. Comparing against
    # the build start time is what makes this a "did THIS run produce it"
    # check rather than a "is a file there" check, so a stale .msi left by an
    # earlier run can never be passed off as this run's output.
    $produced = @()
    if (Test-Path -LiteralPath $BundleMsiDir) {
        $produced = @(Get-ChildItem -LiteralPath $BundleMsiDir -Filter '*.msi' -File |
            Where-Object { $_.LastWriteTimeUtc -gt $startedUtc })
    }

    $expectedNames = @($Language | ForEach-Object { "${productName}_${version}_${arch}_$_.msi" })
    $foundNames = @($produced | ForEach-Object { $_.Name })

    if ($produced.Count -eq 0) {
        $any = if (Test-Path -LiteralPath $BundleMsiDir) {
            @(Get-ChildItem -LiteralPath $BundleMsiDir -Filter '*.msi' -File |
                ForEach-Object { "$($_.Name) (last written $($_.LastWriteTimeUtc.ToString('o')))" })
        } else { @('<bundle directory does not exist>') }
        throw ("tauri build ($scopeName) exited 0 but wrote no new .msi. Expected: " +
               ($expectedNames -join ', ') + ". Present in ${BundleMsiDir}: " + ($any -join '; '))
    }

    $missing = @($expectedNames | Where-Object { $foundNames -notcontains $_ })
    if ($missing.Count -gt 0) {
        throw ("tauri build ($scopeName) exited 0 but did not produce the expected artifact(s): " +
               ($missing -join ', ') + ". Produced instead: " + ($foundNames -join ', ') +
               ". If the Tauri bundler changed its MSI naming scheme, update `$expectedNames in this script.")
    }

    $unexpected = @($foundNames | Where-Object { $expectedNames -notcontains $_ })
    if ($unexpected.Count -gt 0) {
        throw ("tauri build ($scopeName) produced artifact(s) that were not requested: " +
               ($unexpected -join ', '))
    }

    # --- the collision fix, right here -------------------------------------
    foreach ($file in $produced) {
        $lang = $Language | Where-Object { $file.Name -like "*_$_.msi" } | Select-Object -First 1
        $destName = "${productName}_${version}_${arch}_${scopeName}_${lang}.msi"
        $destPath = Join-Path $OutputDir $destName

        if (Test-Path -LiteralPath $destPath) {
            Write-Host "    replacing previous $destName" -ForegroundColor DarkGray
            Remove-Item -LiteralPath $destPath -Force
        }
        Move-Item -LiteralPath $file.FullName -Destination $destPath -Force

        if (Test-Path -LiteralPath $file.FullName) {
            throw "moving $($file.Name) out of the bundle directory failed; the next build would overwrite it"
        }
        Write-Ok "moved $($file.Name) -> $destName"

        $records += Get-ArtifactRecord -Path $destPath -ScopeName $scopeName -Lang $lang
    }
}

# ---------------------------------------------------------------------------
# Verification and record
# ---------------------------------------------------------------------------
Write-Step "artifacts"
foreach ($r in $records) {
    Write-Host ("    {0,-46} {1,12:N0} B  sha256:{2}" -f $r.file, $r.bytes, $r.sha256)
}

$expectedCount = $scopes.Count * $Language.Count
if ($records.Count -ne $expectedCount) {
    throw "expected $expectedCount artifact(s) for $($scopes.Count) scope(s) x $($Language.Count) language(s), recorded $($records.Count)"
}

if (-not $SkipAssertDistinct -and $records.Count -gt 1) {
    Write-Step "distinctness"
    for ($i = 0; $i -lt $records.Count; $i++) {
        for ($j = $i + 1; $j -lt $records.Count; $j++) {
            $a = $records[$i]; $b = $records[$j]
            if ($a.sha256 -eq $b.sha256) {
                throw ("$($a.file) and $($b.file) are byte-identical (sha256 $($a.sha256)). " +
                       "The install scope did not reach the package.")
            }
        }
    }
    # Group by (version, arch, language): within such a group the ONLY intended
    # difference is the scope, and Tauri's own filename has no room for it.
    foreach ($group in ($records | Group-Object { "$($_.file -replace '_per(User|Machine)_', '_SCOPE_')" })) {
        $hashes = @($group.Group | Select-Object -ExpandProperty sha256 -Unique)
        $scopesInGroup = @($group.Group | Select-Object -ExpandProperty scope -Unique)
        if ($scopesInGroup.Count -gt 1 -and $hashes.Count -ne $scopesInGroup.Count) {
            throw ("variants $($scopesInGroup -join ' and ') for '$($group.Name)' are not all distinct " +
                   "($($hashes.Count) distinct hash(es) for $($scopesInGroup.Count) scopes)")
        }
    }
    Write-Ok "$($records.Count) artifacts, all distinct; no filename collision possible"
}

# Cargo.lock: reported, never silently swallowed, never committed here.
$lockAfter = if (Test-Path -LiteralPath $lockFile) {
    (Get-FileHash -LiteralPath $lockFile -Algorithm SHA256).Hash
} else { $null }
if ($lockBefore -ne $lockAfter) {
    Write-Warning ("{0} was rewritten by dependency re-resolution during this build. " +
        "It is NOT committed by this script; commit the refresh deliberately, or the next " +
        "`--locked` build will fail. See docs/reports/2026-10-05-bundle-ci/README.md.") -f $lockFile
}

$manifest = [ordered]@{
    generatedAtUtc = [DateTime]::UtcNow.ToString('o')
    productName    = $productName
    version        = $version
    arch           = $arch
    outputDir      = $OutputDir
    bundleDir      = $BundleMsiDir
    variants       = @($records | Sort-Object scope, language | ForEach-Object {
        [ordered]@{
            scope    = $_.scope
            language = $_.language
            file     = $_.file
            bytes    = $_.bytes
            sha256   = $_.sha256
        }
    })
}
$manifestPath = Join-Path $OutputDir 'manifest.json'
$manifest | ConvertTo-Json -Depth 5 |
    Set-Content -LiteralPath $manifestPath -Encoding UTF8
Write-Ok "wrote $manifestPath"

Write-Step "done: $($records.Count) variant(s) in $OutputDir"
exit 0
