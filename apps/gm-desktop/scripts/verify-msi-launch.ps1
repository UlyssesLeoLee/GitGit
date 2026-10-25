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
    It asserts that the packaged application starts and presents a window
    on a stock Windows machine. It does NOT assert anything about what
    the window shows, that the application works, that it can reach a
    Git server, or that it behaves correctly. "It started" is the whole
    claim, and it is a claim that was previously unmeasured.

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
    [switch]$KeepLogs
)

$ErrorActionPreference = 'Stop'
if (Get-Variable -Name PSNativeCommandUseErrorActionPreference -ErrorAction SilentlyContinue) {
    $PSNativeCommandUseErrorActionPreference = $false
}

function Write-Step { param([string]$M) Write-Host "==> $M" -ForegroundColor Cyan }
function Write-Ok   { param([string]$M) Write-Host "    ok  $M" -ForegroundColor Green }
function Write-Note { param([string]$M) Write-Host "    --  $M" -ForegroundColor DarkGray }

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
