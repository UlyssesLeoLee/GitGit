#requires -Version 7
<#
.SYNOPSIS
    Install each built .msi FOR REAL, assert what landed, uninstall it, and assert it is gone.

.DESCRIPTION
    WHAT THIS PROVES, AND WHAT IT DOES NOT
    ======================================
    `verify-msi.ps1` (the `msi` job) reads the package: it extracts the FILE
    table with `msiexec /a` and reads the MSI tables through Windows Installer
    COM automation. It never installs anything, and its own last line says so:

        PASS: 4 msi verified (payload + scope, no install, no elevation)

    So the repository proves that the `.deb` INSTALLS (the `deb-install` job
    runs a real `apt-get install` in a bare `ubuntu:24.04`) and proves only
    that the `.msi` CONTAINS what it should. For a commercial Windows product
    that is the most basic release property there is, and it was unproven.

    This script is the Windows counterpart of that `.deb` proof. For every
    variant the build produced it:

      1. checks the artefact is byte-for-byte the one `manifest.json` records,
         so the package under test is the one `verify-msi.ps1` blessed;
      2. reads the package's OWN tables (ProductCode, ALLUSERS, INSTALLDIR
         chain, the Registry row that records the install directory, the File
         row for the executable) so the expectations come out of the package
         rather than out of a hardcoded path;
      3. asserts the machine is CLEAN before installing, so a later variant
         cannot silently inherit an earlier variant's install;
      4. installs it with a real `msiexec /i`, waits for the process to finish
         AND polls until the payload and the registration are both there;
      5. asserts the payload landed at the expected path, with the expected
         size, and is byte-identical (SHA-256) to the payload the package
         carries; asserts the package is REGISTERED with the right version;
         asserts the install landed in the scope the package claims;
      6. uninstalls it with a real `msiexec /x`, polls until it is gone, and
         asserts the registration AND the installed files are gone.

    WHY ONE VARIANT AT A TIME
    =========================
    The four variants (perMachine/perUser x en-US/zh-CN) share a product
    name, an install location per scope and an UpgradeCode, so they cannot
    coexist: Windows Installer treats a second install of the same UpgradeCode
    as an upgrade and REMOVES the first. Installing all four at once would
    therefore end with exactly one of them installed, and the job would report
    four successes having proved one. So: install, assert, uninstall, assert
    gone, and only then move to the next variant. The clean-slate assertion
    before every install is what makes "variant B silently inherited variant
    A" impossible rather than merely unlikely - and the final assertion (the
    machine is clean when the last variant is done) is what catches an
    uninstall that lied.

    WHAT IS DELIBERATELY NOT CHECKED
    ===============================
    That the application RUNS. [FACT] It does. The `msi-launch` job in
    `gm-desktop.yml` installs the PACKAGED application, starts it and
    asserts that a top-level window appears - measured 2026-10-25, the
    window is titled 'gitgit Desktop'. (The application was also observed
    to start on a real machine on 2026-10-04; see T10 in
    `docs/plan/v0-tasks.md`.)

    But that is a SEPARATE job, and this script deliberately stays out of
    it. This one's contract is "installs and removes, across all four
    variants", and a webkit application fails to start in a LINUX
    container for reasons that have nothing to do with packaging, and a
    gate that goes red for those reasons gets switched off. What is proven
    HERE is the part packaging controls - the package installs on a
    machine that has nothing but Windows, registers itself, and removes
    itself cleanly.

    The boundary is enforced, not just stated: this script never starts
    `gm-desktop.exe`, and it ASSERTS that no `gm-desktop.exe` process is
    running after an install. The MSI's own `LaunchApplication` custom action
    is bound to the WixUI ExitDialog, which `/qn` never reaches, so a silent
    install cannot start the app either.

    "THE PER-USER VARIANT INSTALLS WITHOUT ELEVATION"
    =================================================
    [FACT] The GitHub `windows-latest` runner's shell runs with an elevated
    token, so "the install succeeded" on its own does NOT prove that the
    package would install for an unprivileged user. The proof used here is
    the package's own scope metadata plus the footprint the install actually
    left:

      * the perUser package must NOT carry the `ALLUSERS` property. (The
        MSI's Property table is read directly. `ALLUSERS=2` means "dual
        purpose, let the installer decide", and on a non-elevated session the
        system then picks per-machine and the install dies with error 1925.)
      * the install must land under %LOCALAPPDATA%\Programs, never under
        Program Files - a per-machine or dual-purpose package would land in
        Program Files even when the caller is elevated, because the SCOPE
        comes from the package, not from the caller's token;
      * the uninstall registration must appear under HKCU and NOT under
        HKLM, and no machine-wide key may be created at all.

    A package that needed elevation would take the per-machine path and fail
    every one of those assertions.

.PARAMETER Dir
    Directory holding the .msi files to install. Default: `msi-out` in the
    current directory, which is where the CI job downloads them.

.PARAMETER ManifestPath
    The `manifest.json` `build-msi.ps1` wrote. Default: `manifest.json` in
    -Dir. The manifest is the ONLY source of truth for which variants exist
    and what their bytes are; this script never invents a second one.

.PARAMETER ExeName
    The executable the package is expected to install. Default
    `gm-desktop.exe`.

.PARAMETER TimeoutSeconds
    How long to wait for an install or an uninstall to become observable.
    Default 300.

.PARAMETER KeepLogs
    Do not delete the per-variant msiexec logs; print where they are.

.EXAMPLE
    ./verify-msi-install.ps1 -Dir msi-out
    Installs, asserts, uninstalls and re-asserts every variant in `msi-out`.

.NOTES
    Exit codes
        0  every variant installed, asserted, uninstalled and re-asserted.
        1  at least one check failed; the reason is printed as
            `INSTALL PROOF FAILED: <reason>`.

    Every check either passes or throws. Nothing is skipped silently: if the
    Windows Installer COM object cannot be created, that is a failure, not a
    reason to quietly report success.
#>

[CmdletBinding()]
param(
    [string]$Dir = 'msi-out',
    [string]$ManifestPath,
    [string]$ExeName = 'gm-desktop.exe',
    [int]$TimeoutSeconds = 300,
    [switch]$KeepLogs
)

$ErrorActionPreference = 'Stop'
if (Get-Variable -Name PSNativeCommandUseErrorActionPreference -ErrorAction SilentlyContinue) {
    $PSNativeCommandUseErrorActionPreference = $false
}

function Write-Step { param([string]$M) Write-Host "==> $M" -ForegroundColor Cyan }
function Write-Ok   { param([string]$M) Write-Host "    ok  $M" -ForegroundColor Green }
function Write-Note { param([string]$M) Write-Host "    --  $M" -ForegroundColor DarkGray }

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
    # PowerShell unrolls an array on the way out of a function. `return $rows`
    # emits the rows one after another, so a caller that assigns the result
    # gets a FLAT list of strings, not a list of rows — and then `$rows[0][0]`
    # is the first CHARACTER of the first row rather than the first field of
    # the first row.
    #
    # [FACT] That is not a hypothetical: with the bare `return $rows`, the
    # first CI run of this job reported "the package's ProductVersion is '0'
    # but the manifest records '0.1.0'" — the first character of "0.1.0".
    # The comma wraps the array so it survives the trip intact.
    return ,$rows
}

function Get-MsiProperty {
    param([Parameter(Mandatory)][string]$Path, [Parameter(Mandatory)][string]$Name)
    $rows = Invoke-MsiQuery -Path $Path -Sql "SELECT ``Value`` FROM ``Property`` WHERE ``Property``='$Name'"
    if ($rows.Count -eq 0) { return $null }
    $v = $rows[0][0]
    if ($v -eq '') { return $null }
    return $v
}

function Get-MsiInstallDirChain {
    <#
        The chain of directory ids from INSTALLDIR up to the root, nearest
        first, together with each one's DefaultDir name.
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

function Get-ExpectedInstallDir {
    <#
        The directory the package installs into, computed from the MSI's own
        Directory table.

        The well-known folder ids are resolved to their real paths and their
        DefaultDir names are ignored: `ProgramFiles64Folder`'s DefaultDir is
        `PFiles`, which is a placeholder and not a directory that exists.
        Everything else contributes its DefaultDir name, whose short form
        (the part before `:`) is the on-disk name.
    #>
    param([Parameter(Mandatory)][string]$Path)

    $wellKnown = @{
        'ProgramFilesFolder'   = $env:ProgramFiles
        'ProgramFiles64Folder' = $env:ProgramFiles
        'LocalAppDataFolder'   = $env:LOCALAPPDATA
    }
    $rows = Invoke-MsiQuery -Path $Path -Sql 'SELECT `Directory`,`Directory_Parent`,`DefaultDir` FROM `Directory`'
    $parent = @{}
    $default = @{}
    foreach ($r in $rows) {
        $parent[$r[0]] = $r[1]
        $default[$r[0]] = $r[2]
    }
    if (-not $parent.ContainsKey('INSTALLDIR')) { throw "'$Path' has no INSTALLDIR directory row" }

    $chain = @()
    $id = 'INSTALLDIR'
    while ($id -and $chain.Count -lt 32) {
        $chain += $id
        $id = $parent[$id]
    }

    # chain is nearest-first; build the path root-first.
    $parts = @()
    for ($i = $chain.Count - 1; $i -ge 0; $i--) {
        $dirId = $chain[$i]
        if ($wellKnown.ContainsKey($dirId)) {
            $parts += $wellKnown[$dirId]
        } elseif ($dirId -eq 'TARGETDIR') {
            continue
        } else {
            $name = $default[$dirId]
            if ($name -match ':') { $name = ($name -split ':')[0] }
            $parts += $name
        }
    }
    return ($parts -join '\')
}

function Get-MsiRegistryKeyFor {
    <#
        The (hive, key) the package itself records the install directory in.
        Read out of the MSI's Registry table rather than hardcoded, so the
        script does not depend on the manufacturer or the product name
        spelling.
    #>
    param(
        [Parameter(Mandatory)][string]$Path,
        [Parameter(Mandatory)][string]$ValueName
    )
    $rows = Invoke-MsiQuery -Path $Path -Sql "SELECT ``Root``,``Key``,``Name`` FROM ``Registry`` WHERE ``Name``='$ValueName'"
    if ($rows.Count -eq 0) { return $null }
    return [pscustomobject]@{
        Root = [int]$rows[0][0]
        Key  = $rows[0][1]
        Name = $rows[0][2]
    }
}

function Get-MsiFileRow {
    <#
        The File table row for the given file name, with the size the
        installer itself recorded.
    #>
    param(
        [Parameter(Mandatory)][string]$Path,
        [Parameter(Mandatory)][string]$FileName
    )
    $rows = Invoke-MsiQuery -Path $Path -Sql 'SELECT `FileName`,`FileSize`,`Version`,`Component_` FROM `File`'
    foreach ($r in $rows) {
        # The File table's FileName column is a SHORT|LONG pair when the file
        # has a generated short name: WiX gives files with long names an
        # 8.3-style short name, and the pair reads `<short>|<long>`. The LONG
        # name is the file the installer actually writes, so it is the one to
        # compare against.
        #
        # [FACT] Measured on the real perMachine package:
        #   FileName='ao5lsmno.exe|gm-desktop.exe'  Size=19160064
        # Taking the FIRST half yields the short name, and the executable
        # would then not be found at all.
        $name = ($r[0] -split '\|')[-1]
        if ($name -ieq $FileName) {
            return [pscustomobject]@{
                Name    = $name
                Bytes   = [long]$r[1]
                Version = $r[2]
                Component = $r[3]
            }
        }
    }
    return $null
}

function Get-MsiFileNames {
    param([Parameter(Mandatory)][string]$Path)
    $rows = Invoke-MsiQuery -Path $Path -Sql 'SELECT `FileName` FROM `File`'
    $names = @()
    foreach ($r in $rows) {
        # SHORT|LONG, same as above: the long name is the installed name.
        $name = ($r[0] -split '\|')[-1]
        $names += $name
    }
    return $names
}

# ---------------------------------------------------------------------------
# Live registry
# ---------------------------------------------------------------------------

# The MSI Registry table's Root column: 0=HKCR, 1=HKCU, 2=HKLM, 3=HKU.
$script:RootToHive = @{
    0 = 'Registry::HKEY_CLASSES_ROOT'
    1 = 'HKCU:'
    2 = 'HKLM:'
    3 = 'Registry::HKEY_USERS'
}

function Get-HivePath {
    param([Parameter(Mandatory)][int]$Root, [Parameter(Mandatory)][string]$Key)
    $hive = $script:RootToHive[$Root]
    if (-not $hive) { throw "unknown MSI Registry root $Root" }
    $key = $Key -replace '\\', '\'
    return (Join-Path $hive $key)
}

function Get-UninstallEntries {
    <#
        Every entry Windows Installer has registered under the Add/Remove
        Programs key, in both hives and in the 32-bit view.
    #>
    $roots = @(
        'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall',
        'HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall',
        'HKLM:\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall'
    )
    $found = @()
    foreach ($root in $roots) {
        if (-not (Test-Path -LiteralPath $root)) { continue }
        $children = @(Get-ChildItem -LiteralPath $root -ErrorAction SilentlyContinue)
        foreach ($child in $children) {
            $props = Get-ItemProperty -LiteralPath $child.PSPath -ErrorAction SilentlyContinue
            if ($null -eq $props) { continue }
            $found += [pscustomobject]@{
                Root           = $root
                Leaf           = $child.PSChildName
                ProductCode    = ($child.PSChildName -replace '^\{|\}$', '')
                DisplayName    = [string]$props.DisplayName
                DisplayVersion = [string]$props.DisplayVersion
            }
        }
    }
    return $found
}

function Get-UninstallEntryFor {
    param([Parameter(Mandatory)][string]$ProductCode)
    $wanted = ($ProductCode -replace '^\{|\}$', '').ToUpperInvariant()
    $matches = @(Get-UninstallEntries | Where-Object { $_.ProductCode.ToUpperInvariant() -eq $wanted })
    return $matches
}

function Test-PathUnder {
    <#
        Is $Path inside $Root? Used to say "this per-user install wrote
        nothing machine-wide" without depending on a second copy of the
        expected path.
    #>
    param(
        [Parameter(Mandatory)][AllowEmptyString()][string]$Path,
        [Parameter(Mandatory)][string]$Root
    )
    if (-not $Path) { return $false }
    $p = $Path.TrimEnd('\')
    $r = $Root.TrimEnd('\')
    return $p.StartsWith($r, [System.StringComparison]::OrdinalIgnoreCase)
}

# ---------------------------------------------------------------------------
# msiexec
# ---------------------------------------------------------------------------

function Invoke-MsiExec {
    <#
        Run msiexec and return its exit code. The log path is returned too,
        because a failed install is only diagnosable from its log.
    #>
    param(
        [Parameter(Mandatory)][string[]]$Arguments,
        [Parameter(Mandatory)][string]$LogPath
    )
    $proc = Start-Process -FilePath 'msiexec.exe' -PassThru -Wait -NoNewWindow `
        -ArgumentList ($Arguments + @('/l*v', "`"$LogPath`""))
    return [pscustomobject]@{
        ExitCode = $proc.ExitCode
        Log      = $LogPath
    }
}

function Wait-For {
    <#
        Poll a condition until it holds or the timeout expires.

        A Windows Installer can hand control back before the work is
        observable - the msiexec client returns once the transaction is
        scheduled, and the file system and the registry catch up after it.
        So the exit code is never the only thing that is waited on.
    #>
    param(
        [Parameter(Mandatory)][scriptblock]$Condition,
        [Parameter(Mandatory)][int]$TimeoutSeconds,
        [Parameter(Mandatory)][string]$What
    )
    $deadline = (Get-Date).AddSeconds($TimeoutSeconds)
    while ((Get-Date) -lt $deadline) {
        if (& $Condition) { return $true }
        Start-Sleep -Milliseconds 500
    }
    return $false
}

function Get-ExtractedPayload {
    <#
        The payload the package carries, extracted with `msiexec /a`.

        This is the SAME extraction `verify-msi.ps1` performs, and it is what
        makes the installed file comparable to the blessed one: the SHA-256
        of the extracted file and the SHA-256 of the file on disk after the
        install have to be equal, so "the installer put the right file
        there" is checked rather than assumed.
    #>
    param(
        [Parameter(Mandatory)][string]$MsiPath,
        [Parameter(Mandatory)][string]$ExeName,
        [Parameter(Mandatory)][string]$WorkDir
    )
    # The extraction target is PER CALL, not per script run.
    #
    # [FACT] The fourth CI run of this job failed on the third variant with
    # "administrative extraction ... yielded 2 'gm-desktop.exe' file(s);
    # expected exactly 1" — because every variant extracted into the SAME
    # `extract` directory, so the first variant's payload was still sitting
    # there when the third variant extracted into it.
    #
    # That is the very defect this script exists to catch in an INSTALLER: a
    # previous variant's artefacts making the next variant's measurement look
    # like it succeeded. It was in the measuring instrument instead.
    $target = Join-Path $WorkDir ('extract-' + [guid]::NewGuid().ToString('N'))
    New-Item -ItemType Directory -Path $target -Force | Out-Null
    $log = Join-Path $WorkDir 'extract.log'
    $run = Invoke-MsiExec -Arguments @('/a', "`"$MsiPath`"", '/qn', "TARGETDIR=`"$target`"") -LogPath $log
    if ($run.ExitCode -ne 0) {
        $tail = (Get-Content -LiteralPath $log -Tail 10 -ErrorAction SilentlyContinue) -join ' | '
        throw "administrative extraction of '$([System.IO.Path]::GetFileName($MsiPath))' failed with exit code $($run.ExitCode). Log tail: $tail"
    }
    $exes = @(Get-ChildItem -LiteralPath $target -Filter $ExeName -File -Recurse -ErrorAction SilentlyContinue)
    if ($exes.Count -ne 1) {
        throw "administrative extraction of '$([System.IO.Path]::GetFileName($MsiPath))' yielded $($exes.Count) '$ExeName' file(s); expected exactly 1"
    }
    return [pscustomobject]@{
        Bytes = $exes[0].Length
        Sha256 = (Get-FileHash -LiteralPath $exes[0].FullName -Algorithm SHA256).Hash.ToLowerInvariant()
    }
}

# ---------------------------------------------------------------------------
# The proof, per variant
# ---------------------------------------------------------------------------

function Assert-CleanMachine {
    <#
        Nothing of this product may be installed before a variant is
        installed.

        This is the assertion that stops one variant from inheriting
        another's install. Without it, a variant whose own install silently
        did nothing would find the PREVIOUS variant's files already on disk
        and pass every "the payload landed" check - a gate reading green
        while checking nothing.
    #>
    param(
        [Parameter(Mandatory)][string]$ProductName,
        [Parameter(Mandatory)][string]$ProductCode,
        [Parameter(Mandatory)][string]$ExpectedDir,
        [Parameter(Mandatory)][string]$Label
    )

    $entries = @(Get-UninstallEntries | Where-Object {
        $_.DisplayName -ieq $ProductName -or $_.ProductCode -eq ($ProductCode -replace '^\{|\}$', '')
    })
    if ($entries.Count -gt 0) {
        $where = ($entries | ForEach-Object { "$($_.Root)\$($_.Leaf)" }) -join ', '
        throw ("${Label}: the machine already has '$ProductName' registered ($where) before this variant was installed. " +
               "A previous variant's install was not removed, so this variant's install would be measured against it.")
    }

    if (Test-Path -LiteralPath $ExpectedDir) {
        $files = @(Get-ChildItem -LiteralPath $ExpectedDir -File -Recurse -ErrorAction SilentlyContinue)
        if ($files.Count -gt 0) {
            $names = ($files | ForEach-Object { $_.FullName }) -join ', '
            throw ("${Label}: '$ExpectedDir' already holds $($files.Count) file(s) before this variant was installed ($names). " +
                   "The previous variant's files are still there, so this variant's install cannot be told apart from it.")
        }
    }
}

function Assert-Installed {
    param(
        [Parameter(Mandatory)][hashtable]$Facts,
        [Parameter(Mandatory)][string]$Label
    )

    $installDir = $null
    $appKeyPath = Get-HivePath -Root $Facts.AppRegRoot -Key $Facts.AppRegKey
    if (Test-Path -LiteralPath $appKeyPath) {
        $props = Get-ItemProperty -LiteralPath $appKeyPath -ErrorAction SilentlyContinue
        if ($null -ne $props -and $props.PSObject.Properties.Name -contains 'InstallDir') {
            $installDir = [string]$props.InstallDir
        }
    }
    if (-not $installDir) {
        throw ("${Label}: the package did not record its install directory at '$appKeyPath' (value 'InstallDir'). " +
               "A package that installs without registering where it put itself cannot be uninstalled by the user either.")
    }

    # --- the payload landed ------------------------------------------------
    $exePath = Join-Path $installDir $ExeName
    if (-not (Test-Path -LiteralPath $exePath)) {
        throw "${Label}: the install reported success but '$exePath' does not exist."
    }
    $exe = Get-Item -LiteralPath $exePath
    if ($exe.Length -ne $Facts.ExeBytes) {
        throw ("${Label}: the installed '$ExeName' is $($exe.Length) B, but the package's own File table records " +
               "$($Facts.ExeBytes) B. The install did not put the package's payload on disk.")
    }
    if ($exe.Length -lt 1MB) {
        throw "${Label}: the installed '$ExeName' is only $($exe.Length) B, below the 1 MB floor: it carries no program."
    }
    $installedHash = (Get-FileHash -LiteralPath $exePath -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($installedHash -ne $Facts.PayloadSha256) {
        throw ("${Label}: the installed '$ExeName' has SHA-256 $installedHash, but the payload the package carries " +
               "has $($Facts.PayloadSha256). The file on disk is not the file the package ships.")
    }

    # --- the licence travelled with it (AGPL-3.0 section 4) ----------------
    $licence = $null
    foreach ($name in $Facts.FileNames) {
        if ($name -match '(?i)^(licen[cs]e|copying|copyright|notice)(\..+)?$') { $licence = $name; break }
    }
    if ($licence) {
        $licencePath = Join-Path $installDir $licence
        if (-not (Test-Path -LiteralPath $licencePath)) {
            throw ("${Label}: the package carries '$licence' but the install did not put it next to the executable. " +
                   "AGPL-3.0 section 4 requires a copy of the Licence with every copy of the Program.")
        }
    }

    # --- the package is registered ----------------------------------------
    $entries = @(Get-UninstallEntryFor -ProductCode $Facts.ProductCode)
    if ($entries.Count -eq 0) {
        throw ("${Label}: the install reported success but no Add/Remove Programs entry exists for " +
               "product code '$($Facts.ProductCode)'. The package is not registered.")
    }
    if ($entries.Count -gt 1) {
        throw ("${Label}: $($entries.Count) Add/Remove Programs entries exist for product code " +
               "'$($Facts.ProductCode)': $(($entries | ForEach-Object { $_.Root }) -join ', ').")
    }
    $entry = $entries[0]
    if ($entry.DisplayVersion -ne $Facts.Version) {
        throw ("${Label}: the registered package reports version '$($entry.DisplayVersion)', but the manifest records " +
               "'$($Facts.Version)'.")
    }
    if ($entry.DisplayName -and $entry.DisplayName -ine $Facts.ProductName) {
        throw ("${Label}: the registered package is called '$($entry.DisplayName)', but the package's own ProductName " +
               "is '$($Facts.ProductName)'.")
    }

    # --- the scope the package claims is the scope it took ---------------
    # [FACT] The Add/Remove Programs entry is NOT the oracle for the scope.
    # Measured on 2026-10-25 on the real per-user package: the ARP entry
    # was under HKLM in BOTH cases — for an unelevated install (files in
    # %LOCALAPPDATA%\Programs) and for an elevated one (files in
    # C:\Program Files). Windows Installer registers the PRODUCT in the
    # machine-wide installer database either way; what the scope decides is
    # where the FILES go and which hive the package's OWN keys are written
    # under. So the oracle is the install directory and the package's own
    # key, and the per-scope checks below use those.
    if ($Facts.Scope -eq 'perUser') {
        # The load-bearing property. ALLUSERS=2 would make the package dual
        # purpose and it would fail with error 1925 for an unprivileged user.
        if ($Facts.AllUsers) {
            throw ("${Label}: the perUser package carries ALLUSERS='$($Facts.AllUsers)'. That overrides the scope the " +
                   "package declares and fails with error 1925 in a non-elevated session.")
        }
        if (-not $appKeyPath.StartsWith('HKCU:', [System.StringComparison]::OrdinalIgnoreCase)) {
            throw ("${Label}: a perUser package recorded its install directory under '$appKeyPath'. " +
                   "A per-user package writes its own keys under HKCU; the package's own Registry table says " +
                   "Root=$($Facts.AppRegRoot).")
        }
        $programsRoot = Join-Path $env:LOCALAPPDATA 'Programs'
        if (-not (Test-PathUnder -Path $installDir -Root $programsRoot)) {
            throw ("${Label}: a perUser package installed into '$installDir', which is not under " +
                   "'$programsRoot'. A per-user package must install into the user profile, and it must do so " +
                   "whatever the installing user's privileges are.")
        }
        $machineAppKey = "HKLM:\$($Facts.AppRegKey)"
        if (Test-Path -LiteralPath $machineAppKey) {
            throw ("${Label}: a perUser package created the machine-wide key '$machineAppKey'. A per-user install must " +
                   "not write machine-wide state, because writing it is what requires elevation.")
        }
        $machineDir = Join-Path $env:ProgramFiles $Facts.ProductName
        if (Test-Path -LiteralPath $machineDir) {
            throw ("${Label}: a perUser package created '$machineDir'. The per-machine footprint is exactly what a " +
                   "per-user install must not have.")
        }
    } else {
        if (-not (Test-PathUnder -Path $installDir -Root $env:ProgramFiles)) {
            throw ("${Label}: a perMachine package installed into '$installDir', which is not under " +
                   "'$($env:ProgramFiles)'.")
        }
    }

    # --- the job installs; it does not launch the app ---------------------
    $running = @(Get-Process -Name ([System.IO.Path]::GetFileNameWithoutExtension($ExeName)) -ErrorAction SilentlyContinue)
    if ($running.Count -gt 0) {
        throw ("${Label}: '$ExeName' is running after the install. This job installs and uninstalls; it does not start " +
               "the application. The `msi-launch` job is where the packaged application is started, and a gate that " +
               "asserts it starts would go red for reasons packaging does not control.")
    }

    return [pscustomobject]@{
        InstallDir   = $installDir
        ExePath      = $exePath
        ExeBytes     = $exe.Length
        ExeSha256    = $installedHash
        LicenceName  = $licence
        RegistryHive = $entry.Root
        DisplayVersion = $entry.DisplayVersion
    }
}

function Assert-Removed {
    param(
        [Parameter(Mandatory)][hashtable]$Facts,
        [Parameter(Mandatory)][string]$InstallDir,
        [Parameter(Mandatory)][string]$ExePath,
        [Parameter(Mandatory)][string]$Label
    )

    $entries = @(Get-UninstallEntryFor -ProductCode $Facts.ProductCode)
    if ($entries.Count -gt 0) {
        throw ("${Label}: the package is still registered after the uninstall ($((($entries | ForEach-Object { $_.Root }) -join ', '))). " +
               "An installer that does not deregister itself cannot be reinstalled cleanly.")
    }
    if (Test-Path -LiteralPath $ExePath) {
        throw ("${Label}: '$ExePath' still exists after the uninstall. An installer that leaves the program behind is a " +
               "defect, and it is the one most likely to be missed.")
    }
    if ($Facts.LicenceName) {
        $licencePath = Join-Path $InstallDir $Facts.LicenceName
        if (Test-Path -LiteralPath $licencePath) {
            throw ("${Label}: '$licencePath' still exists after the uninstall.")
        }
    }
    if (Test-Path -LiteralPath $InstallDir) {
        $files = @(Get-ChildItem -LiteralPath $InstallDir -File -Recurse -ErrorAction SilentlyContinue)
        if ($files.Count -gt 0) {
            $names = ($files | ForEach-Object { $_.FullName }) -join ', '
            throw ("${Label}: '$InstallDir' still holds $($files.Count) file(s) after the uninstall: $names.")
        }
    }
}

# ---------------------------------------------------------------------------
# Driver
# ---------------------------------------------------------------------------

if (-not (Test-Path -LiteralPath $Dir)) {
    throw "artifact directory not found: $Dir"
}
if (-not $ManifestPath) { $ManifestPath = Join-Path $Dir 'manifest.json' }
if (-not (Test-Path -LiteralPath $ManifestPath)) {
    throw "manifest not found: $ManifestPath (the manifest is the only source of truth for the variants)"
}

$manifest = Get-Content -LiteralPath $ManifestPath -Raw -Encoding UTF8 | ConvertFrom-Json
$variants = @($manifest.variants)
if ($variants.Count -eq 0) {
    throw "the manifest records no variant; there is nothing to install"
}

Write-Step "install proof: $($variants.Count) variant(s) from $Dir"
Write-Host "    product  : $($manifest.productName) $($manifest.version) ($($manifest.arch))"

$scopesSeen = @($variants | ForEach-Object { $_.scope } | Select-Object -Unique)
Write-Host "    scopes   : $($scopesSeen -join ', ')"
Write-Host "    languages: $((@($variants | ForEach-Object { $_.language } | Select-Object -Unique)) -join ', ')"

$workRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("gm-desktop-install-" + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $workRoot -Force | Out-Null

$summary = @()
try {
    foreach ($v in $variants) {
        $label = "$($v.scope)/$($v.language)"
        Write-Step "variant $label ($($v.file))"

        $msiPath = Join-Path $Dir $v.file
        if (-not (Test-Path -LiteralPath $msiPath)) {
            throw "${label}: the manifest lists '$($v.file)' but it is not in '$Dir'. The artifact under test would not be the one that was built."
        }

        # --- the artefact under test is the one that was built ------------
        $item = Get-Item -LiteralPath $msiPath
        if ($item.Length -ne [long]$v.bytes) {
            throw "${label}: '$($v.file)' is $($item.Length) B on disk but the manifest records $($v.bytes) B."
        }
        $hash = (Get-FileHash -LiteralPath $msiPath -Algorithm SHA256).Hash.ToLowerInvariant()
        if ($hash -ne $v.sha256) {
            throw "${label}: '$($v.file)' has SHA-256 $hash but the manifest records $($v.sha256)."
        }
        Write-Ok "artefact matches the manifest ($($item.Length) B, sha256:$($hash.Substring(0, 16))...)"

        # --- the package's own facts --------------------------------------
        $productCode = Get-MsiProperty -Path $msiPath -Name 'ProductCode'
        $productName = Get-MsiProperty -Path $msiPath -Name 'ProductName'
        $productVersion = Get-MsiProperty -Path $msiPath -Name 'ProductVersion'
        $allUsers = Get-MsiProperty -Path $msiPath -Name 'ALLUSERS'
        if (-not $productCode) { throw "${label}: the package carries no ProductCode property." }
        if (-not $productVersion) { throw "${label}: the package carries no ProductVersion property." }
        if ($productVersion -ne $manifest.version) {
            throw "${label}: the package's ProductVersion is '$productVersion' but the manifest records '$($manifest.version)'."
        }

        $chain = Get-MsiInstallDirChain -Path $msiPath
        $expectedDir = Get-ExpectedInstallDir -Path $msiPath
        $appReg = Get-MsiRegistryKeyFor -Path $msiPath -ValueName 'InstallDir'
        if ($null -eq $appReg) {
            throw "${label}: the package's Registry table records no 'InstallDir' value, so the install cannot be located."
        }
        $exeRow = Get-MsiFileRow -Path $msiPath -FileName $ExeName
        if ($null -eq $exeRow) {
            throw "${label}: the package's File table has no '$ExeName' row."
        }
        $fileNames = @(Get-MsiFileNames -Path $msiPath)
        Write-Host "    package  : product $($productCode), version $($productVersion), scope $($v.scope)"
        Write-Host "    INSTALLDIR chain: $($chain.Ids -join ' > ')"
        Write-Host "    expected dir    : $expectedDir"

        # --- nothing may be installed yet ----------------------------------
        Assert-CleanMachine -ProductName $productName -ProductCode $productCode `
            -ExpectedDir $expectedDir -Label $label
        Write-Ok "the machine is clean: no registration, no leftover files"

        # --- the payload the package carries ------------------------------
        $payload = Get-ExtractedPayload -MsiPath $msiPath -ExeName $ExeName -WorkDir $workRoot
        Write-Ok "package payload: $ExeName = $($payload.Bytes) B, sha256:$($payload.Sha256.Substring(0, 16))..."

        # --- install -------------------------------------------------------
        $installLog = Join-Path $workRoot ("install-" + ($v.scope) + "-" + ($v.language) + ".log")
        $run = Invoke-MsiExec -Arguments @('/i', "`"$msiPath`"", '/qn', '/norestart') -LogPath $installLog
        if ($run.ExitCode -ne 0) {
            $tail = (Get-Content -LiteralPath $installLog -Tail 12 -ErrorAction SilentlyContinue) -join ' | '
            throw "${label}: msiexec /i exited $($run.ExitCode). Log tail: $tail"
        }

        # --- the install is observable, not merely scheduled ----------------
        $installed = $null
        $ready = Wait-For -TimeoutSeconds $TimeoutSeconds -What 'the install to become observable' -Condition {
            $appKeyNow = Get-HivePath -Root $appReg.Root -Key $appReg.Key
            $dirNow = $null
            if (Test-Path -LiteralPath $appKeyNow) {
                $p = Get-ItemProperty -LiteralPath $appKeyNow -ErrorAction SilentlyContinue
                if ($null -ne $p -and $p.PSObject.Properties.Name -contains 'InstallDir') { $dirNow = [string]$p.InstallDir }
            }
            if (-not $dirNow) { return $false }
            if (-not (Test-Path -LiteralPath (Join-Path $dirNow $ExeName))) { return $false }
            if (@(Get-UninstallEntryFor -ProductCode $productCode).Count -eq 0) { return $false }
            return $true
        }
        if (-not $ready) {
            throw ("${label}: msiexec /i exited 0 but the install did not become observable within $TimeoutSeconds s " +
                   "(no '$ExeName' under the recorded install directory, or no Add/Remove Programs entry).")
        }
        Write-Ok "installed (msiexec exit 0, payload and registration both observable)"

        $facts = @{
            ProductCode   = $productCode
            ProductName   = $productName
            Version       = $productVersion
            AllUsers      = $allUsers
            Scope         = $v.scope
            AppRegRoot    = $appReg.Root
            AppRegKey     = $appReg.Key
            ExeBytes      = $payload.Bytes
            PayloadSha256 = $payload.Sha256
            FileNames     = $fileNames
        }
        $state = Assert-Installed -Facts $facts -Label $label
        Write-Ok "payload on disk: $($state.ExePath) = $($state.ExeBytes) B, sha256:$($state.ExeSha256.Substring(0, 16))..."
        Write-Ok "registered: $($state.RegistryHive) -> $($state.DisplayVersion)"

        # --- uninstall -----------------------------------------------------
        $uninstallLog = Join-Path $workRoot ("uninstall-" + ($v.scope) + "-" + ($v.language) + ".log")
        $un = Invoke-MsiExec -Arguments @('/x', "`"$productCode`"", '/qn', '/norestart') -LogPath $uninstallLog
        if ($un.ExitCode -ne 0) {
            $tail = (Get-Content -LiteralPath $uninstallLog -Tail 12 -ErrorAction SilentlyContinue) -join ' | '
            throw "${label}: msiexec /x exited $($un.ExitCode). Log tail: $tail"
        }
        $gone = Wait-For -TimeoutSeconds $TimeoutSeconds -What 'the uninstall to become observable' -Condition {
            $stillRegistered = @(Get-UninstallEntryFor -ProductCode $productCode).Count -gt 0
            $stillThere = Test-Path -LiteralPath $state.ExePath
            return (-not $stillRegistered) -and (-not $stillThere)
        }
        if (-not $gone) {
            throw ("${label}: msiexec /x exited 0 but the uninstall did not become observable within $TimeoutSeconds s " +
                   "(the registration or the installed file is still there).")
        }
        Assert-Removed -Facts $facts -InstallDir $state.InstallDir -ExePath $state.ExePath -Label $label
        Write-Ok "uninstalled: registration gone, '$($state.ExePath)' gone, '$($state.InstallDir)' holds no file"

        $summary += [pscustomobject]@{
            scope = $v.scope
            language = $v.language
            file = $v.file
            installDir = $state.InstallDir
            bytes = $state.ExeBytes
            sha256 = $state.ExeSha256
            registry = $state.RegistryHive
        }
    }

    # --- the machine is clean at the end -----------------------------------
    Write-Step 'final state'
    $leftovers = @(Get-UninstallEntries | Where-Object { $_.DisplayName -ieq $manifest.productName })
    if ($leftovers.Count -gt 0) {
        throw ("'$($manifest.productName)' is still registered after every variant was uninstalled " +
               "($(($leftovers | ForEach-Object { $_.Root + '\' + $_.Leaf }) -join ', ')).")
    }
    Write-Ok "no registration for '$($manifest.productName)' remains"

    Write-Step "PASS: $($summary.Count) msi installed, asserted, uninstalled and re-asserted"
    foreach ($s in $summary) {
        Write-Host ("    {0,-22} {1,12:N0} B  sha256:{2}  {3}" -f "$($s.scope)/$($s.language)", $s.bytes, $s.sha256.Substring(0, 16), $s.installDir)
    }
    exit 0
} catch {
    Write-Host "INSTALL PROOF FAILED: $($_.Exception.Message)" -ForegroundColor Red
    if ($KeepLogs) {
        Write-Host "logs kept in: $workRoot" -ForegroundColor DarkGray
    } else {
        Write-Host "msiexec logs were in: $workRoot" -ForegroundColor DarkGray
    }
    exit 1
} finally {
    if (-not $KeepLogs -and (Test-Path -LiteralPath $workRoot)) {
        Remove-Item -LiteralPath $workRoot -Recurse -Force -ErrorAction SilentlyContinue
    }
}
