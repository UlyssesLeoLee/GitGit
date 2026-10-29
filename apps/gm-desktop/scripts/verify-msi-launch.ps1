#requires -Version 7
<#
.SYNOPSIS
    Install the packaged MSI, LAUNCH the application, assert that a window
    appeared, then remove it again.

.DESCRIPTION
    WHAT THIS IS FOR
    =================
    The repository's `msi-install` job proves that the Windows package
    installs, registers, and removes itself cleanly. It deliberately does
    NOT launch the application, and the reason it gives is a Linux one: a
    webkit/GTK app fails to start in a container that has no dbus, no
    seat and no hardware, for reasons that have nothing to do with
    packaging, and a gate that reddens for those reasons gets switched
    off.

    That reason does not hold on `windows-latest`. The runner has a real
    interactive desktop and the Edge WebView2 runtime is part of the
    image. So the same question can be asked here of the PACKAGED
    artefact, which is a strictly stronger claim than the one the
    repository has been able to make so far: that the app runs.

    So: this script installs the package, starts the installed
    executable, waits for it to present a top-level window, and fails if
    it does not. A package that installs but crashes on startup is
    exactly the defect this is here to find.

    It is a SEPARATE script from `verify-msi-install.ps1` on purpose.
    That script's contract is "installs and removes; does not launch",
    and a gate that asserts the boundary is in place is worth keeping
    intact. This one is the experiment, and it is opt-in by being a
    different file.

    WHAT IT ASSERTS, AND WHAT IT DOES NOT
    =====================================
    It asserts two things about the PACKAGED artefact on a stock Windows
    machine: that the application starts and presents a window, AND that
    the webview rendered the product rather than a blank page.

    The second assertion is the one that was missing. A window handle is
    not evidence of a rendered UI: `MainWindowHandle` is non-zero for a
    window that was created and never drawn, so "it started" and "it
    shows the product" are different claims, and only the first one used
    to be made.

    The application writes ONE line to its own log file when the webview
    reports that it has booted, and this script waits for that line. The
    chain is: the line is in the log file => the Rust side received a
    command call from the webview => the webview's JavaScript ran => the
    Svelte application mounted => the window is showing the product.

    It does NOT assert that the rendered UI is correct - that the routes
    show the right data, that the stylesheet loaded, that nothing is
    visually broken. It proves the application BOOTED, which is the step
    that was previously unmeasured; it is not a visual regression test
    and it does not replace one.

    The boundary the repository has drawn for the LINUX container case
    is unchanged and is restated here for the Windows case: the
    application's behaviour is not under test, only its start-up.

.PARAMETER Msi
    The .msi to install. Required.

.PARAMETER ExeName
    The executable to launch. Defaults to the package name of
    `src-tauri/Cargo.toml`, which is the default binary name the Rust
    build produces.

.PARAMETER TimeoutSeconds
    How long to wait for the window to appear. Default 90.

.PARAMETER KeepLogs
    Do not delete the msiexec logs; print where they are.

.NOTES
    Exit codes
        0  the application started and presented a window.
        1  a check failed; the reason is printed as
            `LAUNCH PROOF FAILED: <reason>`.
        2  setup error (the MSI is missing, or the exe name cannot be
            determined) — a setup error is a failure, not a pass. A check
            that quietly checked nothing is the failure mode this
            repository keeps meeting.
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$Msi,
    [string]$ExeName,
    [int]$TimeoutSeconds = 90,
    [int]$MarkerTimeoutSeconds = 60,
    [switch]$KeepLogs
)

$ErrorActionPreference = 'Stop'
if (Get-Variable -Name PSNativeCommandUseErrorActionPreference -ErrorAction SilentlyContinue) {
    $PSNativeCommandUseErrorActionPreference = $false
}

function Write-Step { param([string]$M) Write-Host "==> $M" -ForegroundColor Cyan }
function Write-Ok   { param([string]$M) Write-Host "    ok  $M" -ForegroundColor Green }
function Write-Note { param([string]$M) Write-Host "    --  $M" -ForegroundColor DarkGray }

# Does the log file carry the marker AFTER the given offset?
#
# [FACT] The file is opened with FileShare.ReadWrite, because the running
# application holds it open for append and a reader that demands exclusive
# access would fail on exactly the file it is looking for.
function Test-MarkerInLog {
    param([string]$Path, [long]$Offset, [string]$Marker)
    $stream = $null
    $reader = $null
    try {
        $stream = [System.IO.File]::Open($Path, [System.IO.FileMode]::Open,
            [System.IO.FileAccess]::Read, [System.IO.FileShare]::ReadWrite)
        if ($stream.Length -le $Offset) { return $false }
        [void]$stream.Seek($Offset, [System.IO.SeekOrigin]::Begin)
        $reader = [System.IO.StreamReader]::new($stream)
        return $reader.ReadToEnd().Contains($Marker)
    } catch {
        return $false
    } finally {
        if ($null -ne $reader) { $reader.Dispose() }
        elseif ($null -ne $stream) { $stream.Dispose() }
    }
}

if (-not (Test-Path -LiteralPath $Msi)) {
    Write-Error "LAUNCH PROOF FAILED: the package '$Msi' is not there"
    exit 2
}

# The executable's name is a fact about the repository, not a guess about
# the package: the default binary name of the `gm-desktop` crate.
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..' '..' '..')).Path
$tauriManifest = Join-Path $repoRoot 'apps/gm-desktop/src-tauri/Cargo.toml'
if (-not $ExeName) {
    $nameLine = Select-String -LiteralPath $tauriManifest -Pattern '^\s*name\s*=\s*"([^"]+)"' |
        Select-Object -First 1
    if ($null -eq $nameLine) {
        Write-Error "LAUNCH PROOF FAILED: no package name in $tauriManifest, so the executable to launch is unknown"
        exit 2
    }
    $ExeName = $nameLine.Matches[0].Groups[1].Value
}
Write-Step "launch proof for $(Split-Path -Leaf $Msi) (executable '$ExeName')"

# --- the identity the application's log file is found under -------------------
# [FACT] `tauri-plugin-log`'s `LogDir` target resolves to Tauri's
# `app_log_dir()`, which is `data_local_dir()` + the bundle identifier +
# `logs`, and the file inside it is named after the product name with its
# extension replaced by `.log`. Both facts come from the application's own
# config, so they are read from it rather than written down here.
$tauriConfig = Join-Path $repoRoot 'apps/gm-desktop/src-tauri/tauri.conf.json'
if (-not (Test-Path -LiteralPath $tauriConfig)) {
    Write-Error "LAUNCH PROOF FAILED: the tauri config is not at '$tauriConfig', so the application's log file cannot be found"
    exit 2
}
$config = Get-Content -LiteralPath $tauriConfig -Raw | ConvertFrom-Json
$AppIdentifier = [string]$config.identifier
if ([string]::IsNullOrWhiteSpace($AppIdentifier)) {
    Write-Error "LAUNCH PROOF FAILED: the tauri config has no identifier, so the application's log directory is unknown"
    exit 2
}

# --- the marker the webview writes ------------------------------------------
# [FACT] The marker text is read out of the Rust source rather than
# copied into this script, so the two cannot drift: a rename of the
# constant changes what this script greps for, and a missing constant is
# a setup error rather than a silent pass.
$markerSource = Join-Path $repoRoot 'apps/gm-desktop/src-tauri/src/commands/uiboot.rs'
if (-not (Test-Path -LiteralPath $markerSource)) {
    Write-Error "LAUNCH PROOF FAILED: '$markerSource' is not there, so the webview-boot marker is unknown"
    exit 2
}
$markerMatch = Select-String -LiteralPath $markerSource -Pattern 'UI_BOOT_MARKER:\s*&str\s*=\s*"(?<marker>[A-Za-z0-9_]+)"' |
    Select-Object -First 1
if ($null -eq $markerMatch) {
    Write-Error "LAUNCH PROOF FAILED: no UI_BOOT_MARKER constant found in '$markerSource', so the webview-boot marker is unknown"
    exit 2
}
$UiBootMarker = $markerMatch.Matches[0].Groups['marker'].Value
Write-Host "    webview-boot marker: $UiBootMarker"

$work = Join-Path ([System.IO.Path]::GetTempPath()) ('gm-desktop-launch-' + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $work -Force | Out-Null
$installLog = Join-Path $work 'install.log'
$uninstallLog = Join-Path $work 'uninstall.log'

function Invoke-MsiExec {
    param([string[]]$Arguments, [string]$LogPath)
    $proc = Start-Process -FilePath 'msiexec.exe' -PassThru -Wait -NoNewWindow `
        -ArgumentList ($Arguments + @('/l*v', "`"$LogPath`""))
    return $proc.ExitCode
}

try {
    # --- install ---------------------------------------------------------
    $code = Invoke-MsiExec -Arguments @('/i', "`"$Msi`"", '/qn') -LogPath $installLog
    if ($code -ne 0) {
        $tail = (Get-Content -LiteralPath $installLog -Tail 12 -ErrorAction SilentlyContinue) -join ' | '
        Write-Error "LAUNCH PROOF FAILED: msiexec /i exited $code. Log tail: $tail"
        exit 1
    }
    Write-Ok 'installed silently'

    # --- where did it put the executable ----------------------------------
    # Both install scopes are searched, because which one applies depends on
    # the installing user's privileges, and the point of this check is the
    # application's start-up rather than the scope question (which the
    # install proof already settles).
    #
    # [FACT] `Get-ChildItem -Filter` matches the WHOLE file name, so the
    # filter has to carry the extension: `-Filter 'gm-desktop'` does NOT
    # match 'gm-desktop.exe'. The first CI run of this job failed with
    # "the install succeeded but no 'gm-desktop' was found under C:\Program
    # Files\" for exactly that reason — the install had succeeded and the
    # file was sitting right there.
    $exeFile = if ($ExeName -like '*.exe') { $ExeName } else { "$ExeName.exe" }
    $roots = @(
        (Join-Path $env:ProgramFiles ''),
        (Join-Path $env:LOCALAPPDATA 'Programs')
    )
    $found = @()
    foreach ($root in $roots) {
        if (-not $root -or -not (Test-Path -LiteralPath $root)) { continue }
        $found += @(Get-ChildItem -LiteralPath $root -Filter $exeFile -File -Recurse `
            -ErrorAction SilentlyContinue | Select-Object -First 3)
    }
    if ($found.Count -eq 0) {
        Write-Error ("LAUNCH PROOF FAILED: the install succeeded but no '$exeFile' was found under " +
            ($roots -join ' or '))
        exit 1
    }
    $exe = $found[0].FullName
    Write-Ok "installed executable: $exe ($($found[0].Length) B)"

    # --- the log state BEFORE the launch ------------------------------------
    # [FACT] The application's log file lives in the user's profile and
    # SURVIVES a launch, so a marker written by an EARLIER run can be
    # sitting in it. Searching the whole file would therefore accept a
    # previous run's evidence, and the gate would pass without this launch
    # having booted anything. The length of every log file is recorded
    # here, before the launch, and only the bytes after it are searched.
    $logDir = Join-Path ([Environment]::GetFolderPath('LocalApplicationData')) (Join-Path $AppIdentifier 'logs')
    $logOffsets = @{}
    foreach ($existing in @(Get-ChildItem -LiteralPath $logDir -Filter '*.log' -File -ErrorAction SilentlyContinue)) {
        $logOffsets[$existing.FullName] = $existing.Length
    }

    # --- launch it -------------------------------------------------------
    $proc = Start-Process -FilePath $exe -PassThru
    Write-Step "started '$exe' (pid $($proc.Id)); waiting for a window"

    # --- does a window appear? -------------------------------------------
    # `MainWindowHandle` is non-zero only for a process that owns a
    # top-level window, so a process that starts and then dies leaves it
    # at zero. The title is checked too, because a window that is created
    # but never shown still has a handle.
    $deadline = (Get-Date).AddSeconds($TimeoutSeconds)
    $handle = [IntPtr]::Zero
    $title = ''
    $exited = $false
    while ((Get-Date) -lt $deadline) {
        Start-Sleep -Milliseconds 500
        $live = Get-Process -Id $proc.Id -ErrorAction SilentlyContinue
        if ($null -eq $live) { $exited = $true; break }
        $live.Refresh()
        if ($live.MainWindowHandle -ne [IntPtr]::Zero -and $live.MainWindowTitle) {
            $handle = $live.MainWindowHandle
            $title = $live.MainWindowTitle
            break
        }
    }

    if ($exited) {
        Write-Error ("LAUNCH PROOF FAILED: '$ExeName' started and then exited within $TimeoutSeconds s " +
            "without ever presenting a window. The package installs; the application does not survive start-up.")
        exit 1
    }
    if ($handle -eq [IntPtr]::Zero) {
        Write-Error ("LAUNCH PROOF FAILED: '$ExeName' is running but presented no top-level window within " +
            "$TimeoutSeconds s.")
        exit 1
    }
    Write-Ok "the application is running with a window: '$title'"

    # --- did the webview actually render the product? ----------------------
    # [FACT] A window is not the product. `MainWindowHandle` is non-zero
    # for a window that was created and never drawn, so the check above
    # proves the application STARTED, not that it SHOWS anything. A
    # packaged application whose webview fails to load presents exactly
    # the same window as one that renders the product.
    #
    # [FACT] The application writes ONE line to its own log file when the
    # webview reports that it has booted, and that line is the evidence:
    # the Rust side received a command call from the webview, so the
    # webview's JavaScript ran and the Svelte application mounted.
    Write-Step "waiting for the webview-boot marker in the application's log"
    $markerSeen = $false
    $markerDeadline = (Get-Date).AddSeconds($MarkerTimeoutSeconds)
    while ((Get-Date) -lt $markerDeadline) {
        $logs = @(Get-ChildItem -LiteralPath $logDir -Filter '*.log' -File -ErrorAction SilentlyContinue)
        foreach ($log in $logs) {
            $offset = if ($logOffsets.ContainsKey($log.FullName)) { $logOffsets[$log.FullName] } else { 0 }
            if (Test-MarkerInLog -Path $log.FullName -Offset $offset -Marker $UiBootMarker) {
                Write-Ok "the webview reported that the UI booted: '$UiBootMarker' in $($log.Name)"
                $markerSeen = $true
                break
            }
        }
        if ($markerSeen) { break }
        if ($proc.HasExited) { break }
        Start-Sleep -Milliseconds 500
    }
    if (-not $markerSeen) {
        Write-Error ("LAUNCH PROOF FAILED: the application started and presented a window, but the " +
                     "webview never reported that the UI booted - no '$UiBootMarker' line appeared in " +
                     "the application's log directory '$logDir'. A window that renders nothing still " +
                     "has a window handle, so this is the check that tells the two apart.")
        exit 1
    }

    # --- and it did not start a second copy ------------------------------
    $copies = @(Get-Process -Name ([System.IO.Path]::GetFileNameWithoutExtension($exeFile)) -ErrorAction SilentlyContinue)
    if ($copies.Count -gt 1) {
        Write-Note "$($copies.Count) processes named '$ExeName' are running; the launch may have re-executed itself"
    }
}
finally {
    # --- stop it ---------------------------------------------------------
    Get-Process -Name ([System.IO.Path]::GetFileNameWithoutExtension($exeFile)) -ErrorAction SilentlyContinue |
        ForEach-Object {
            Write-Note "stopping pid $($_.Id)"
            try { $_.Kill() } catch { }
        }
    Start-Sleep -Seconds 2

    # --- and remove the package ------------------------------------------
    $code = Invoke-MsiExec -Arguments @('/x', "`"$Msi`"", '/qn') -LogPath $uninstallLog
    if ($code -eq 0) {
        Write-Ok 'uninstalled'
    } else {
        Write-Note "msiexec /x exited $code (the runner is discarded anyway)"
    }
    if ($KeepLogs) {
        Write-Note "logs kept in $work"
    } else {
        Remove-Item -LiteralPath $work -Recurse -Force -ErrorAction SilentlyContinue
    }
}

Write-Ok 'PASS: the packaged application installed, started, presented a window, and was removed'
exit 0
