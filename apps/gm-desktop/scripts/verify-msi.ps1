#requires -Version 7
<#
.NOTES
    AGPL-3.0 section 4 IS CHECKED HERE, AND THE CHECK IS AN ASSERTION.

    The requirement: a copy of the Licence has to travel with every copy of
    the Program. The MSI carries it twice over.

    1. As the install-time licence dialog. `bundle.licenseFile` reaches the
       installer because `crates/tauri-bundler/src/bundle/windows/msi/mod.rs`
       converts the licence text to RTF, writes it to `<out>/wix/LICENSE.rtf`
       and inserts a `license` key into the Handlebars data map. The same map
       is rendered into a CUSTOM template - the bundler registers either this
       repository's `wix/main.wxs` or its own built-in copy, then renders both
       with the same `data` - so `{{license}}` is available here. This
       template already used it for `WixUILicenseRtf`, and the `{{#unless
       license}}` block below that skips the licence dialog is not taken
       because the licence IS set.

    2. As an installed file. `wix/main.wxs` adds a `LICENSE.rtf` file to the
       `Path` component, so the licence lands next to `gm-desktop.exe` in
       INSTALLDIR and is removed with it. `main-peruser.wxs` is generated from
       `main.wxs` by `scripts/render-wix-template.py`, so both scopes get it.

    THE EARLIER CLAIM IN THIS FILE WAS WRONG, AND THE CORRECTION MATTERS.
    It used to say that configuration could not fix this, on the grounds that
    a hand-written template receives no licence material. That is false: the
    `license` key is in the data map for custom templates too. The reason the
    omission went unnoticed is a measurement artefact rather than a bundler
    limitation: `msiexec /a` extracts the package's FILE table, and the RTF
    lives in the installer's UI, not in that table. Run 37642251415 extracted
    `admin-install.log`, the .msi itself and `PFiles\gitgit Desktop\
    gm-desktop.exe` - and reported "no licence" - while the licence was in
    fact being displayed at install time.

    [FACT] The check below is on the extracted FILE table, because that is
    what a recipient can inspect afterwards. An installer that shows a licence
    dialog and installs nothing beside the executable would still fail it.

    [INFERENCE] The MSI cannot ship until a signing certificate is configured,
    so this is downstream of that blocker rather than an additional one.

.SYNOPSIS
    Prove a built .msi is well-formed, carries payload, and really is the
    install scope its filename claims - without installing anything and
    without administrator rights.

.DESCRIPTION
    Two independent checks per .msi, then cross-variant checks.

    1. ADMINISTRATIVE-INSTALL EXTRACTION (`msiexec /a`)
       `msiexec /a <msi> /qn TARGETDIR=<tmp>` performs an administrative
       install: Windows Installer resolves and lays out the package's file
       table into TARGETDIR but registers nothing, writes no registry values,
       creates no shortcuts and needs no elevation. It is the cheapest way to
       get the real payload out of an MSI. The check asserts that exactly one
       `gm-desktop.exe` is present and that it is a plausible size, which
       distinguishes "a real installer" from "a package that links nothing".

    2. MSI TABLE READS (Windows Installer COM automation)
       The payload check cannot tell the two scopes apart - both embed the
       same executable - so the scope is read out of the package itself, via
       the tables that carry it:
         * `Directory`: the chain of parents above `INSTALLDIR`. Per-machine
           must be under Program Files; per-user must be under
           %LOCALAPPDATA%\Programs. This is the structural difference and it
           is what a silently mis-scoped package would get wrong.
         * `Property`: `ALLUSERS`. A per-user package must NOT carry it. This
           is load-bearing, not cosmetic: `ALLUSERS=2` means "dual purpose,
           let the installer decide at run time", and on a non-elevated
           session the system then picks per-machine and the install dies with
           error 1925. A per-user package therefore has to leave the property
           absent, which is only expressible through `InstallScope`.

    3. CROSS-VARIANT
       All variants must coexist in one directory under distinct names with
       distinct SHA-256s, and the two scopes' `INSTALLDIR` parents must
       differ. Same filename twice is exactly the failure this repo is
       guarding against, so it is asserted rather than assumed.

       If `manifest.json` sits next to the artifacts (build-msi.ps1 writes
       it), the recorded size and hash of each variant are re-checked against
       the file on disk.

    Every check either passes or throws. Nothing is skipped silently: if the
    Windows Installer COM object cannot be created, that is a failure, not a
    reason to quietly report success.

.PARAMETER Dir
    Directory holding the .msi files to verify. Default: the `msi-out`
    directory under `src-tauri/target`, which is where build-msi.ps1 writes.

.PARAMETER Msi
    Explicit .msi paths. When given, -Dir is not used and no manifest is
    consulted. Useful for checking one archived artifact.

.PARAMETER ExeName
    Main executable the package is expected to contain. Default:
    `gm-desktop.exe`.

.PARAMETER MinExeBytes
    Smallest acceptable size for that executable. Default 1 MB, far below the
    real binary, so the check fails only on a package that embeds nothing.

.PARAMETER MinLicenceBytes
    Smallest acceptable size for the installed licence file. Default 1 KB,
    which is far below the AGPL-3.0 text (about 34 KB) and far above an empty
    or truncated file, so the check fails only on a file that conveys nothing.

.PARAMETER KeepExtract
    Do not delete the extraction directories, and print where they are.

.EXAMPLE
    ./verify-msi.ps1
    Verifies every .msi in `src-tauri/target/msi-out`.

.EXAMPLE
    ./verify-msi.ps1 -Msi 'C:\artifacts\gitgit Desktop_0.1.0_x64_perUser_en-US.msi'
    Verifies one archived installer.

.NOTES
    Exit codes
        0  every .msi passed every check.
        1  at least one check failed; the reason is printed as
            `VERIFY FAILED: <reason>`.

    Scope comes from the FILENAME, not from a manifest: the scope-qualified
    name is the artifact contract build-msi.ps1 establishes, and reading the
    scope back out of it is what makes a mis-named artifact a failure. When
    -Msi is used with names that carry no scope token, the scope-specific
    assertions are skipped and only the payload and table checks run; the
    skip is printed.
#>

[CmdletBinding()]
param(
    [string]$Dir,
    [string[]]$Msi,
    [string]$ExeName = 'gm-desktop.exe',
    [long]$MinExeBytes = 1MB,
    [long]$MinLicenceBytes = 1KB,
    [switch]$KeepExtract
)

$ErrorActionPreference = 'Stop'
if (Get-Variable -Name PSNativeCommandUseErrorActionPreference -ErrorAction SilentlyContinue) {
    $PSNativeCommandUseErrorActionPreference = $false
}

$AppDir   = Split-Path -Parent $PSScriptRoot
$TauriDir = Join-Path $AppDir 'src-tauri'
if (-not $Dir) { $Dir = Join-Path $TauriDir 'target\msi-out' }

# Scope expectations, read back out of the artifact name. `$Scope` is $null
# when the name carries no scope token, which downgrades the scope-specific
# assertions to "print and move on" - explicitly, never silently.
$PerMachineParentPattern = 'ProgramFiles'
$PerUserParentPattern    = 'LocalAppData'

function Write-Step { param([string]$M) Write-Host "==> $M" -ForegroundColor Cyan }
function Write-Ok   { param([string]$M) Write-Host "    ok  $M" -ForegroundColor Green }

# ---------------------------------------------------------------------------
# Windows Installer COM automation
# ---------------------------------------------------------------------------
$script:Installer = $null

function Get-MsiDatabase {
    param([Parameter(Mandatory)][string]$Path)
    if ($null -eq $script:Installer) {
        try {
            $script:Installer = New-Object -ComObject WindowsInstaller.Installer
        } catch {
            throw ("could not create the WindowsInstaller.Installer COM object, so the MSI " +
                   "tables cannot be read and the scope of '$Path' cannot be verified: $($_.Exception.Message)")
        }
    }
    $flags = [System.Reflection.BindingFlags]
    $db = $script:Installer.GetType().InvokeMember(
        'OpenDatabase', $flags::InvokeMethod, $null, $script:Installer, @($Path, 0))
    return $db
}

function Invoke-MsiQuery {
    <#
        Run a SQL SELECT against an MSI and return rows as string arrays.
        MSI identifiers are quoted with backticks; in a single-quoted
        PowerShell string a backtick is an ordinary character, so they are
        written literally.
    #>
    param(
        [Parameter(Mandatory)][string]$Path,
        [Parameter(Mandatory)][string]$Sql
    )
    $flags = [System.Reflection.BindingFlags]
    $db = Get-MsiDatabase -Path $Path
    $view = $db.GetType().InvokeMember('OpenView', $flags::InvokeMethod, $null, $db, @($Sql))
    $view.GetType().InvokeMember('Execute', $flags::InvokeMethod, $null, $view, $null) | Out-Null
    $rows = @()
    while ($true) {
        $rec = $view.GetType().InvokeMember('Fetch', $flags::InvokeMethod, $null, $view, $null)
        if ($null -eq $rec) { break }
        $count = $rec.GetType().InvokeMember('FieldCount', $flags::GetProperty, $null, $rec, $null)
        $row = @()
        for ($i = 1; $i -le $count; $i++) {
            $row += [string]$rec.GetType().InvokeMember('StringData', $flags::GetProperty, $null, $rec, $i)
        }
        $rows += ,$row
    }
    $view.GetType().InvokeMember('Close', $flags::InvokeMethod, $null, $view, $null) | Out-Null
    # `return ,$rows` and NOT `return $rows`.
    #
    # PowerShell unrolls an array on the way out of a function, and the
    # unrolling bites in exactly one case: when the query matched a SINGLE
    # row, the caller is handed that row's fields as a flat list instead of
    # a list of rows, and `$rows[0][0]` is then the first CHARACTER of the
    # first field rather than the field itself.
    #
    # [FACT] The only property this script reads is ALLUSERS, whose value is
    # the single character "1", so the bug has been invisible here. It is
    # not invisible in general: `verify-msi-install.ps1` reads
    # ProductVersion, and on its first CI run it reported the package's
    # ProductVersion as '0' — the first character of "0.1.0". The comma
    # wraps the array so the rows survive the trip intact, in both the
    # single-row and the multi-row case.
    return ,$rows
}

function Get-MsiProperty {
    param([Parameter(Mandatory)][string]$Path, [Parameter(Mandatory)][string]$Name)
    $rows = Invoke-MsiQuery -Path $Path -Sql "SELECT ``Value`` FROM ``Property`` WHERE ``Property``='$Name'"
    if ($rows.Count -eq 0) { return $null }
    return $rows[0][0]
}

function Get-InstallDirChain {
    <#
        The chain of directory ids from INSTALLDIR up to the root, nearest
        first, together with each one's DefaultDir name. This is the
        structural evidence of install scope.
    #>
    param([Parameter(Mandatory)][string]$Path)
    $rows = Invoke-MsiQuery -Path $Path -Sql 'SELECT `Directory`,`Directory_Parent`,`DefaultDir` FROM `Directory`'
    $parent = @{}
    $default = @{}
    foreach ($r in $rows) {
        $parent[$r[0]] = $r[1]
        $default[$r[0]] = $r[2]
    }
    if (-not $parent.ContainsKey('INSTALLDIR')) {
        throw "'$Path' has no INSTALLDIR directory row; this is not a Tauri-produced MSI"
    }
    $chain = @()
    $id = 'INSTALLDIR'
    while ($id -and $chain.Count -lt 32) {
        $chain += $id
        $id = $parent[$id]
    }
    return [pscustomobject]@{
        Ids    = $chain
        Names  = @($chain | ForEach-Object { $default[$_] })
        Parent = $parent['INSTALLDIR']
    }
}

# ---------------------------------------------------------------------------
# Check 1b: the Corresponding Source notice (AGPL-3.0 section 6(d))
# ---------------------------------------------------------------------------

# The place the Corresponding Source is offered at. It is derived from the
# same config the bundler reads, so a notice that names a different place
# does not pass.
$SourcePlace = $null
$confPath = Join-Path $PSScriptRoot '../src-tauri/tauri.conf.json'
if (-not (Test-Path -LiteralPath $confPath)) {
    throw "the tauri config was not found at '$confPath', so the place the Corresponding Source is offered at cannot be established."
}
$conf = Get-Content -LiteralPath $confPath -Raw | ConvertFrom-Json
if (-not $conf.bundle.homepage) {
    throw "the tauri config has no 'bundle.homepage', so the place the Corresponding Source is offered at cannot be established."
}
$SourcePlace = "$($conf.bundle.homepage)/releases"

# Find the notice among the extracted files, BY ITS CONTENT.
#
# [FACT] The path and the file name are deliberately not pinned. A check
# pinned to one filename goes red on a compliant package the moment someone
# renames it, and such a gate gets switched off. Three markers are required,
# and they are the ones the notice itself carries: it has to say what it is
# about, it has to name the licence, and it has to name the place the source
# is offered at.
function Find-CorrespondingSourceNotice {
    param(
        [Parameter(Mandatory)][string]$Root,
        [Parameter(Mandatory)][string]$Place
    )
    # [FACT] The size bound keeps the payload out of the way without pinning
    # a file name: the notice is a couple of kilobytes and the executable is
    # megabytes.
    $candidates = @(Get-ChildItem -LiteralPath $Root -File -Recurse -ErrorAction SilentlyContinue |
        Where-Object { $_.Length -gt 0 -and $_.Length -lt 1MB })
    foreach ($c in $candidates) {
        $text = Get-Content -LiteralPath $c.FullName -Raw -ErrorAction SilentlyContinue
        if ($null -eq $text) { continue }
        if ($text -notlike '*Corresponding Source for this build*') { continue }
        if ($text -notlike '*AGPL-3.0*') { continue }
        if ($text -notmatch [regex]::Escape($Place)) { continue }
        return $c
    }
    return $null
}

# ---------------------------------------------------------------------------
# Check 1: administrative install (no install, no elevation)
# ---------------------------------------------------------------------------
function Test-AdminInstall {
    param([Parameter(Mandatory)][string]$Path)

    $target = Join-Path ([System.IO.Path]::GetTempPath()) ("gm-desktop-verify-" + [guid]::NewGuid().ToString('N'))
    New-Item -ItemType Directory -Path $target -Force | Out-Null
    $log = Join-Path $target 'admin-install.log'

    try {
        # /qn = no UI, /a = administrative install, /l*v = verbose log. This
        # registers nothing and does not require elevation.
        $proc = Start-Process -FilePath 'msiexec.exe' -PassThru -Wait -NoNewWindow -ArgumentList @(
            '/a', "`"$Path`"", '/qn', "TARGETDIR=`"$target`"", '/l*v', "`"$log`""
        )
        $code = $proc.ExitCode
        if ($code -ne 0) {
            $tail = (Get-Content -LiteralPath $log -Tail 12 -ErrorAction SilentlyContinue) -join ' | '
            throw "administrative install of '$([System.IO.Path]::GetFileName($Path))' failed with exit code $code. Log tail: $tail"
        }

        $exes = @(Get-ChildItem -LiteralPath $target -Filter $ExeName -File -Recurse -ErrorAction SilentlyContinue)
        if ($exes.Count -ne 1) {
            $all = @(Get-ChildItem -LiteralPath $target -File -Recurse -ErrorAction SilentlyContinue)
            throw ("administrative install of '$([System.IO.Path]::GetFileName($Path))' yielded " +
                   "$($exes.Count) '$ExeName' file(s); expected exactly 1. " +
                   "$($all.Count) file(s) were extracted in total.")
        }
        $exe = $exes[0]
        if ($exe.Length -lt $MinExeBytes) {
            throw ("$($exe.Name) extracted from '$([System.IO.Path]::GetFileName($Path))' is " +
                   "$($exe.Length) B, below the $MinExeBytes B floor: the package carries no real payload.")
        }
        $files = @(Get-ChildItem -LiteralPath $target -File -Recurse -ErrorAction SilentlyContinue)

        # AGPL-3.0 section 4: a copy of the Licence has to travel with every
        # copy of the Program.
        #
        # The exact path is deliberately NOT pinned. The requirement is that a
        # licence-bearing file is installed at all; a check pinned to one
        # filename goes red on a compliant package the moment someone renames
        # it, and such a gate gets switched off.
        #
        # The contents are checked too, because a file merely NAMED like a
        # licence would satisfy a name-only check while conveying nothing.
        $licences = @(Get-ChildItem -LiteralPath $target -File -Recurse -ErrorAction SilentlyContinue |
            Where-Object { $_.Name -match '(?i)^(licen[cs]e|copying|copyright|notice)(\..+)?$' })
        if ($licences.Count -eq 0) {
            $names = ($files | ForEach-Object { $_.FullName.Substring($target.Length).TrimStart('\', '/') }) -join ', '
            throw ("administrative install of '$([System.IO.Path]::GetFileName($Path))' extracted " +
                   "$($files.Count) file(s) and not one of them is a licence, copyright, copying or notice " +
                   "file. AGPL-3.0 section 4 requires a copy of the Licence with every copy of the " +
                   "Program. Extracted: $names. The mechanism is the LicenseFile in the Path component of " +
                   "wix/main.wxs (and the main-peruser.wxs generated from it), fed by bundle.licenseFile.")
        }
        $licence = $licences[0]
        if ($licence.Length -lt $MinLicenceBytes) {
            throw ("the licence file '$($licence.Name)' installed by '$([System.IO.Path]::GetFileName($Path))' is " +
                   "$($licence.Length) B, below the $MinLicenceBytes B floor: it conveys nothing.")
        }
        $licenceText = Get-Content -LiteralPath $licence.FullName -Raw -ErrorAction SilentlyContinue
        # Two phrases, not one. A file that merely MENTIONS the licence would
        # satisfy a title-only check - this repository's own README.md carries
        # the licence title twice - so the check also wants a phrase that only
        # the licence text itself carries.
        if ($licenceText -notmatch 'GNU AFFERO GENERAL PUBLIC LICENSE' -or
            $licenceText -notmatch 'TERMS AND CONDITIONS') {
            throw ("the licence file '$($licence.Name)' installed by '$([System.IO.Path]::GetFileName($Path))' does " +
                   "not contain the AGPL-3.0 text. A file with a licence-shaped name, or one that merely " +
                   "mentions the licence, is not a licence.")
        }

        $rel = $exe.FullName.Substring($target.Length).TrimStart('\', '/')

        # AGPL-3.0 section 6(d): the Corresponding Source notice, found by
        # its content among the extracted files.
        $notice = Find-CorrespondingSourceNotice -Root $target -Place $SourcePlace
        $noticeRel = $null
        $noticeBytes = 0
        if ($notice) {
            $noticeRel = $notice.FullName.Substring($target.Length).TrimStart('\', '/')
            $noticeBytes = $notice.Length
        }
        return [pscustomobject]@{
            ExeBytes      = $exe.Length
            ExeRelPath    = $rel
            FileCount     = $files.Count
            LicenceName   = $licence.Name
            LicenceBytes  = $licence.Length
            LicenceRelPath = $licence.FullName.Substring($target.Length).TrimStart('\', '/')
            Notice        = $notice
            NoticeRelPath = $noticeRel
            NoticeBytes   = $noticeBytes
        }
    } finally {
        if ($KeepExtract) {
            Write-Host "    extraction kept: $target" -ForegroundColor DarkGray
        } elseif (Test-Path -LiteralPath $target) {
            Remove-Item -LiteralPath $target -Recurse -Force -ErrorAction SilentlyContinue
        }
    }
}

# ---------------------------------------------------------------------------
# Driver
# ---------------------------------------------------------------------------
if ($Msi) {
    $files = @($Msi | ForEach-Object { Get-Item -LiteralPath $_ })
} else {
    if (-not (Test-Path -LiteralPath $Dir)) {
        throw "artifact directory not found: $Dir (run build-msi.ps1 first)"
    }
    $files = @(Get-ChildItem -LiteralPath $Dir -Filter '*.msi' -File | Sort-Object Name)
    if ($files.Count -eq 0) {
        throw "no .msi found in $Dir"
    }
}

Write-Step "verify $($files.Count) msi in $(if ($Msi) { '(explicit paths)' } else { $Dir })"
foreach ($f in $files) { Write-Host "    $($f.Name)  $($f.Length) B  sha256:$((Get-FileHash -LiteralPath $f.FullName -Algorithm SHA256).Hash.ToLowerInvariant())" }

$results = @()

foreach ($f in $files) {
    Write-Step "check $($f.Name)"

    $scope = if ($f.Name -match '_perMachine_') { 'perMachine' }
             elseif ($f.Name -match '_perUser_') { 'perUser' }
             else { $null }
    if ($null -eq $scope) {
        Write-Host "    NOTE: filename carries no scope token; scope assertions skipped for this file" -ForegroundColor Yellow
    } else {
        Write-Host "    scope from filename: $scope"
    }

    # --- payload, no install / no elevation
    $admin = Test-AdminInstall -Path $f.FullName
    Write-Ok "payload: $($admin.ExeRelPath) = $($admin.ExeBytes) B, $($admin.FileCount) file(s) extracted"
    Write-Ok "licence: $($admin.LicenceRelPath) = $($admin.LicenceBytes) B (contains the AGPL-3.0 text)"

    # --- AGPL-3.0 section 6(d): the Corresponding Source notice
    #
    # [FACT] The per-MACHINE package carries the notice, and it is ASSERTED
    # here, by content.
    #
    # [FACT] The per-USER package does NOT carry it, and that is a known
    # gap rather than an oversight. The bundler renders every
    # `bundle.resources` entry as a component whose KeyPath is a FILE, and
    # ICE38 rejects a component that installs into the user profile whose
    # KeyPath is a file: `light.exe` fails with `LGHT0204 : ICE38` and
    # exits 204. The per-user WiX template therefore does not render the
    # resources at all — which keeps that build working, and leaves the
    # notice out of that one variant. The gap is recorded in
    # `docs/plan/v0-tasks.md`; it is not a claim that the variant complies.
    if ($scope -eq 'perMachine') {
        if ($null -eq $admin.Notice) {
            $extracted = ($files | ForEach-Object { $_.FullName.Substring($target.Length).TrimStart('\', '/') }) -join ', '
            throw ("$($f.Name) ships no Corresponding Source notice next to the object code. " +
                   "AGPL-3.0 section 6(d) requires the object code to carry written directions " +
                   "saying where to find the Corresponding Source. The mechanism is " +
                   "bundle.resources, which the MSI bundler renders as a resource component. " +
                   "Extracted: $extracted")
        }
        Write-Ok "corresponding source notice: $($admin.NoticeRelPath) = $($admin.NoticeBytes) B"
    } else {
        Write-Host "    NOTE: this variant does not carry the Corresponding Source notice (ICE38 keeps a file keypath out of the user profile); see docs/plan/v0-tasks.md" -ForegroundColor Yellow
    }

    # --- scope, read from the package itself
    $chain = Get-InstallDirChain -Path $f.FullName
    Write-Host "    INSTALLDIR chain: $($chain.Ids -join ' > ')"
    Write-Host "    DefaultDir names: $($chain.Names -join ' | ')"

    if ($scope -eq 'perMachine' -and ($chain.Ids -join ' ') -notmatch $PerMachineParentPattern) {
        throw ("$($f.Name) claims perMachine but its INSTALLDIR chain is '$($chain.Ids -join ' > ')', " +
               "which does not contain '$PerMachineParentPattern'.")
    }
    if ($scope -eq 'perUser' -and ($chain.Ids -join ' ') -notmatch $PerUserParentPattern) {
        throw ("$($f.Name) claims perUser but its INSTALLDIR chain is '$($chain.Ids -join ' > ')', " +
               "which does not contain '$PerUserParentPattern'.")
    }

    $allUsers = Get-MsiProperty -Path $f.FullName -Name 'ALLUSERS'
    if ($scope -eq 'perUser') {
        # See .DESCRIPTION: ALLUSERS=2 overrides the scope WiX wrote and
        # produces error 1925 in a non-elevated session.
        if ($allUsers -and $allUsers -ne '') {
            throw ("$($f.Name) is a perUser package but carries ALLUSERS='$allUsers'. " +
                   "That overrides InstallScope and fails with error 1925 in a non-elevated session.")
        }
        Write-Ok "perUser: ALLUSERS absent (the load-bearing per-user invariant holds)"
    } else {
        Write-Host "    ALLUSERS = $(if ($null -eq $allUsers) { '<absent>' } else { "'$allUsers'" })" -ForegroundColor DarkGray
    }

    $results += [pscustomobject]@{
        file            = $f.Name
        scope           = $scope
        bytes           = $f.Length
        sha256          = (Get-FileHash -LiteralPath $f.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
        exeBytes        = $admin.ExeBytes
        licenceBytes    = $admin.LicenceBytes
        installDirChain = ($chain.Ids -join ' > ')
        allUsers        = if ($null -eq $allUsers) { '<absent>' } else { $allUsers }
    }
}

# ---------------------------------------------------------------------------
# Cross-variant checks
# ---------------------------------------------------------------------------
Write-Step 'cross-variant'
$names = @($results | ForEach-Object { $_.file })
if (($names | Select-Object -Unique).Count -ne $names.Count) {
    throw "two results share a filename: $($names -join ', ')"
}
$hashes = @($results | Select-Object -ExpandProperty sha256 -Unique)
if ($hashes.Count -ne $results.Count) {
    throw "two variants are byte-identical; the scope did not reach the package"
}
Write-Ok "$($results.Count) distinct filenames, $($hashes.Count) distinct hashes, all coexisting in one directory"

$pm = @($results | Where-Object { $_.scope -eq 'perMachine' })
$pu = @($results | Where-Object { $_.scope -eq 'perUser' })
if ($pm.Count -gt 0 -and $pu.Count -gt 0) {
    $pmChains = @($pm | Select-Object -ExpandProperty installDirChain -Unique)
    $puChains = @($pu | Select-Object -ExpandProperty installDirChain -Unique)
    foreach ($c in $pmChains) {
        foreach ($u in $puChains) {
            if ($c -eq $u) { throw "perMachine and perUser packages share the INSTALLDIR chain '$c'" }
        }
    }
    Write-Ok "perMachine INSTALLDIR: $($pmChains -join ' ; ')"
    Write-Ok "perUser    INSTALLDIR: $($puChains -join ' ; ')"
} else {
    Write-Host "    NOTE: only one scope present; the cross-scope comparison did not run" -ForegroundColor Yellow
}

# Manifest cross-check: the recorded hash must still match the file on disk.
$manifestPath = Join-Path $Dir 'manifest.json'
if (-not $Msi -and (Test-Path -LiteralPath $manifestPath)) {
    $manifest = Get-Content -LiteralPath $manifestPath -Raw -Encoding UTF8 | ConvertFrom-Json
    foreach ($v in $manifest.variants) {
        $actual = $results | Where-Object { $_.file -eq $v.file }
        if (-not $actual) { throw "manifest lists '$($v.file)' but it is not among the verified files" }
        if ($actual.sha256 -ne $v.sha256) {
            throw "manifest hash for '$($v.file)' ($($v.sha256)) does not match the file on disk ($($actual.sha256))"
        }
        if ([long]$v.bytes -ne [long]$actual.bytes) {
            throw "manifest size for '$($v.file)' ($($v.bytes) B) does not match the file on disk ($($actual.bytes) B)"
        }
    }
    Write-Ok "manifest.json: $($manifest.variants.Count) recorded variant(s) match the files on disk"
}

Write-Step "PASS: $($results.Count) msi verified (payload + scope, no install, no elevation)"
exit 0
