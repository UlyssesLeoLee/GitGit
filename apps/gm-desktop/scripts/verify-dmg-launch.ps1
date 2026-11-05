#requires -Version 7
<#
.SYNOPSIS
    Mount the built .dmg read-only, LAUNCH the application out of the mounted
    volume, and assert that its webview reported a boot.

.DESCRIPTION
    WHAT THIS IS FOR
    =================
    The `dmg` job in `.github/workflows/gm-desktop-bundle.yml` builds the
    macOS disk image and OPENS it: `hdiutil imageinfo` parses the image, it
    is mounted read-only, and the `.app` inside it is checked for a real
    Mach-O binary (>= 1 MiB) and for the manifest's version. That is a
    statement about the PACKAGE.

    It was never a statement about the APPLICATION. The application inside
    the image had never been started on any machine, and the repository
    said so in plain words (`README.md`: the macOS/Linux GUI has never run).
    The `.msi` lane had already broken that symmetry on Windows:
    `verify-msi-launch.ps1` installs the packaged application, launches it,
    and waits for the webview to report that it booted.

    This script is the macOS counterpart of that proof, asked of the
    artefact that has never been observed.

    WHAT IT ASSERTS, AND WHAT IT DOES NOT
    =====================================
    It asserts ONE thing: the application, started from the mounted `.dmg`,
    reported a boot through its webview.

    The chain is the one the Windows lane already proves. The application
    writes ONE line to its OWN log file when the webview reports that the
    Svelte application mounted
    (`apps/gm-desktop/src-tauri/src/commands/uiboot.rs`). The line is in
    the log file => the Rust side received a command call from the webview
    => the webview's JavaScript ran => the Svelte application mounted =>
    the window is showing the product rather than a blank page.

    It does NOT assert that the rendered UI is correct - that the routes
    show the right data, that the stylesheet loaded, that the window looks
    right. The Windows lane has a pixel criterion
    (`apps/gm-desktop/scripts/window-capture.ps1`); macOS has no
    `PrintWindow`, and reproducing a window capture is a separate piece of
    work that is deliberately OUT of scope here. This lane proves the
    application BOOTED. It is not a visual regression test and it does not
    replace one. Do not read "the app booted" as "the app looks right".

    THE ONE-SHOT TOKEN
    ==================
    The application's log file lives in the user's profile and SURVIVES a
    launch, so a marker written by an EARLIER run can be sitting in it, and
    a check that greps for a constant marker can be satisfied by history -
    it would pass without this launch having booted anything.

    So the launcher mints a token before it starts the application and
    hands it over through the environment
    (`GITGIT_UI_BOOT_NONCE`); only the process that was given THIS token
    can write a line that matches THIS launch. The search can therefore
    cover the whole log file without caring where in it the evidence sits.
    That matters because `tauri-plugin-log` rotates its file at
    `max_file_size` (1 MiB here) by DELETING it and creating a fresh one,
    and it does that at start-up when the old file is already at the limit:
    a "length before the launch" rule would point past the end of the new,
    nearly empty file, and a healthy launch would be reported as a failure.

    The decision half - minting the token and searching for the marker that
    carries it - is the SAME code the Windows lane uses, in
    `msi-launch-log.ps1` (the file name says `msi`; the logic is
    platform-independent, and `test-msi-launch-log.ps1` exercises it with
    synthetic log files in the always-running `gate` workflow). This file
    dot-sources it rather than re-implementing it, so the two platforms
    cannot drift apart on the one thing that decides whether a marker is
    evidence.

    WHERE THE LOG GOES
    ==================
    `tauri-plugin-log`'s `LogDir` target resolves to Tauri's
    `app_log_dir()`. On macOS that is
    `dirs::home_dir()/Library/Logs/<bundle identifier>` - read out of
    `tauri.conf.json`'s `identifier` rather than written down here, so a
    rename of the identifier moves the log with it. The file inside is
    named after the product name with `.log` appended, i.e.
    `gitgit Desktop.log`.

    The candidates are searched in that order, and the one that actually
    carried the marker is printed. The alternatives are listed only so
    that a failure message can say where the log WAS, rather than only
    where it was expected to be.

    GATEKEEPER
    ==========
    Nothing here disables Gatekeeper, clears the quarantine attribute, or
    re-signs the application. The `.dmg` is ad-hoc signed
    (`APPLE_SIGNING_IDENTITY: '-'`) and not notarized, and that is what a
    user receives. A CI runner that had Gatekeeper switched off would
    prove something a user's machine does not have, so the script runs the
    application exactly as it is and reports what macOS says.

    THE FAILURE PATH IS PART OF THE PROOF
    ======================================
    `-SelfTestRedPath` runs this script WITHOUT starting the application.
    The proof must then fail - the marker cannot appear, because the only
    process that could write it was never started - and the script exits
    NON-ZERO with the `LAUNCH PROOF FAILED` message. That is what makes
    the green run meaningful: a gate that has never been shown to fail is
    not known to be a gate.

    The Linux container case is untouched by any of this: `deb-install`
    still deliberately does not start a window, because a webkit/GTK
    application fails there for reasons that have nothing to do with
    packaging.

.PARAMETER DmgPath
    The `.dmg` to mount and launch out of. Required.

.PARAMETER MarkerTimeoutSeconds
    How long to wait for the boot marker to appear. Default 90.

.PARAMETER SelfTestRedPath
    Do not start the application. The proof is then expected to FAIL, and
    the script exits non-zero with the `LAUNCH PROOF FAILED` message.
    Used by the CI step that proves this gate can go red.

.NOTES
    Exit codes
        0  the application was started out of the mounted image and its
           webview reported a boot.
        1  the proof failed: the application did not start, or it started
           and the webview never reported a boot. The reason is printed as
           `LAUNCH PROOF FAILED: <reason>`.
        2  setup error - the image is missing, cannot be mounted, or the
           repository's own configuration cannot be read. A setup error is
           a failure, not a pass: a check that quietly checked nothing is
           the failure mode this repository keeps meeting.
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$DmgPath,
    [int]$MarkerTimeoutSeconds = 90,
    [switch]$SelfTestRedPath
)

$ErrorActionPreference = 'Stop'
if (Get-Variable -Name PSNativeCommandUseErrorActionPreference -ErrorAction SilentlyContinue) {
    $PSNativeCommandUseErrorActionPreference = $false
}

function Write-Step { param([string]$M) Write-Host "==> $M" -ForegroundColor Cyan }
function Write-Ok   { param([string]$M) Write-Host "    ok  $M" -ForegroundColor Green }
function Write-Note { param([string]$M) Write-Host "    --  $M" -ForegroundColor DarkGray }
function Write-Bad  { param([string]$M) Write-Host "    !!  $M" -ForegroundColor Red }

# The decision half of this proof - the one-shot token and the search for the
# marker that carries it - is shared with the Windows lane. See the file
# header for why it is not re-implemented here.
$launchLogHelper = Join-Path $PSScriptRoot 'msi-launch-log.ps1'
if (-not (Test-Path -LiteralPath $launchLogHelper)) {
    Write-Bad "LAUNCH PROOF FAILED: the launch-proof helper '$launchLogHelper' is not there"
    exit 2
}
. $launchLogHelper

if (-not (Test-Path -LiteralPath $DmgPath)) {
    Write-Bad "LAUNCH PROOF FAILED: the disk image '$DmgPath' is not there"
    exit 2
}

# --- the repository's own facts, read rather than written down ---------------
# [FACT] The bundle identifier is what Tauri's `app_log_dir()` puts the
# application's log file under, and the binary's name is what the `.app`
# bundle's `Contents/MacOS` directory holds. Both come from the
# repository's own files, so a rename moves the proof with it instead of
# leaving it searching for a path that no longer exists.
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..' '..' '..')).Path
$tauriConfig = Join-Path $repoRoot 'apps/gm-desktop/src-tauri/tauri.conf.json'
$tauriManifest = Join-Path $repoRoot 'apps/gm-desktop/src-tauri/Cargo.toml'
$markerSource = Join-Path $repoRoot 'apps/gm-desktop/src-tauri/src/commands/uiboot.rs'

foreach ($required in @($tauriConfig, $tauriManifest, $markerSource)) {
    if (-not (Test-Path -LiteralPath $required)) {
        Write-Bad "LAUNCH PROOF FAILED: '$required' is not there, so the application cannot be located"
        exit 2
    }
}

$config = Get-Content -LiteralPath $tauriConfig -Raw | ConvertFrom-Json
$AppIdentifier = [string]$config.identifier
if ([string]::IsNullOrWhiteSpace($AppIdentifier)) {
    Write-Bad "LAUNCH PROOF FAILED: the tauri config has no identifier, so the application's log directory is unknown"
    exit 2
}
$ProductName = [string]$config.productName

$nameLine = Select-String -LiteralPath $tauriManifest -Pattern '^\s*name\s*=\s*"([^"]+)"' |
    Select-Object -First 1
if ($null -eq $nameLine) {
    Write-Bad "LAUNCH PROOF FAILED: no package name in $tauriManifest, so the executable to launch is unknown"
    exit 2
}
$BinaryName = $nameLine.Matches[0].Groups[1].Value

# [FACT] The marker text is read out of the Rust source rather than copied
# into this script, so the two cannot drift: a rename of the constant
# changes what this script greps for, and a missing constant is a setup
# error rather than a silent pass.
$markerMatch = Select-String -LiteralPath $markerSource -Pattern 'UI_BOOT_MARKER:\s*&str\s*=\s*"(?<marker>[A-Za-z0-9_]+)"' |
    Select-Object -First 1
if ($null -eq $markerMatch) {
    Write-Bad "LAUNCH PROOF FAILED: no UI_BOOT_MARKER constant found in '$markerSource', so the webview-boot marker is unknown"
    exit 2
}
$UiBootMarker = $markerMatch.Matches[0].Groups['marker'].Value

Write-Step "launch proof for $(Split-Path -Leaf $DmgPath) (binary '$BinaryName', identifier '$AppIdentifier')"
Write-Host "    webview-boot marker constant: $UiBootMarker"

# --- where the application's own log file is --------------------------------
# [FACT] `tauri-plugin-log`'s `LogDir` target resolves to Tauri's
# `app_log_dir()`, which on macOS is `dirs::home_dir()/Library/Logs/<bundle
# identifier>` (tauri `src/path/desktop.rs`, the `#[cfg(target_os = "macos")]`
# arm of `app_log_dir`). The identifier is read from `tauri.conf.json`
# above, so this is a fact about the application rather than a guess.
#
# The second and third candidates are not Tauri's path on macOS; they are
# there so that a failure message can say where the log WAS if the first
# one is empty, instead of only where it was expected to be.
$homeDir = if ($env:HOME) { $env:HOME } else { [Environment]::GetFolderPath('UserProfile') }
$logDirCandidates = @(
    (Join-Path $homeDir (Join-Path 'Library' (Join-Path 'Logs' $AppIdentifier))),
    (Join-Path $homeDir (Join-Path 'Library' (Join-Path 'Logs' (Join-Path $AppIdentifier 'logs')))),
    (Join-Path $homeDir (Join-Path 'Library' (Join-Path 'Application Support' (Join-Path $AppIdentifier 'logs'))))
)

# --- the scratch directory, the mount point, and the app's output ----------
$work = Join-Path ([System.IO.Path]::GetTempPath()) ('gm-desktop-dmg-launch-' + [guid]::NewGuid().ToString('N'))
$mountDir = Join-Path $work 'volume'
$appStdout = Join-Path $work 'app.stdout'
$appStderr = Join-Path $work 'app.stderr'
$hdiStdout = Join-Path $work 'hdiutil.stdout'
$hdiStderr = Join-Path $work 'hdiutil.stderr'
$eulaAnswers = Join-Path $work 'eula-answers.txt'

$mounted = $false
$appProcess = $null
$setupError = $false
$exitCode = 0

try {
    New-Item -ItemType Directory -Path $mountDir -Force | Out-Null

    # `hdiutil` is part of macOS. It is checked for explicitly so that a
    # runner without it is reported as a SETUP error (exit 2) rather than
    # as a failed proof (exit 1): the difference is whether the check
    # looked at the application or never got as far as looking at it.
    if (-not (Get-Command -Name 'hdiutil' -ErrorAction SilentlyContinue)) {
        $setupError = $true
        throw 'hdiutil is not on this machine, so the disk image cannot be opened at all'
    }

    # --- mount the image read-only ------------------------------------------
    # [FACT] `bundle.licenseFile` puts the AGPL into the image as a UDIF
    # EULA resource, and `hdiutil` refuses to mount an image that carries
    # one until the agreement is accepted: a non-interactive runner has
    # nobody to accept it, so the attach is cancelled
    # ("hdiutil: attach canceled"). `verify-bundle.sh` answers it the same
    # way, and answering it is not a workaround - it accepts the licence
    # this repository ships, on our own artefact.
    #
    # Sixty-four "y" lines is more than the prompt consumes; the surplus
    # is harmless because the pipe buffer takes it and `hdiutil` stops
    # reading.
    Set-Content -LiteralPath $eulaAnswers -Value (1..64 | ForEach-Object { 'y' })
    $answers = Get-Content -LiteralPath $eulaAnswers
    $answers | & hdiutil attach -readonly -nobrowse -mountpoint $mountDir $DmgPath 1> $hdiStdout 2> $hdiStderr
    $attachCode = $LASTEXITCODE
    $attachOut = (Get-Content -LiteralPath $hdiStdout -Raw -ErrorAction SilentlyContinue)
    $attachErr = (Get-Content -LiteralPath $hdiStderr -Raw -ErrorAction SilentlyContinue)
    if ($attachCode -ne 0) {
        $setupError = $true
        throw ("hdiutil attach exited $attachCode.`n" +
            "hdiutil stdout: $attachOut`n" +
            "hdiutil stderr: $attachErr`n" +
            "The image could not be opened read-only. Failing closed: an image we cannot open " +
            "proves nothing. (If this is the runner refusing to mount rather than a bad artefact, " +
            "that is a finding, not something to work around silently.)")
    }
    $mounted = $true
    Write-Ok "mounted read-only at $mountDir"

    # --- the application inside it ------------------------------------------
    $appBundle = Get-ChildItem -LiteralPath $mountDir -Directory -Filter '*.app' -ErrorAction SilentlyContinue |
        Select-Object -First 1
    if ($null -eq $appBundle) {
        $setupError = $true
        $listing = (Get-ChildItem -LiteralPath $mountDir -Force -ErrorAction SilentlyContinue |
            ForEach-Object { $_.Name }) -join ', '
        throw "the mounted image contains no .app bundle (it holds: $listing)"
    }
    $appExec = Join-Path $appBundle.FullName (Join-Path 'Contents' (Join-Path 'MacOS' $BinaryName))
    if (-not (Test-Path -LiteralPath $appExec)) {
        $setupError = $true
        throw "the mounted $($appBundle.Name) has no Contents/MacOS/$BinaryName"
    }
    Write-Ok "application bundle: $($appBundle.FullName)"

    # --- this launch's one-shot token ---------------------------------------
    $bootNonce = New-BootNonce
    Set-Item -Path "env:$($script:BootNonceVariable)" -Value $bootNonce
    $UiBootMarkerText = Get-BootMarkerText -Marker $UiBootMarker -Nonce $bootNonce
    Write-Host "    webview-boot marker: $UiBootMarkerText"

    if ($SelfTestRedPath) {
        # The red path: the application is NOT started, so the only process
        # that could write this launch's marker was never started, and the
        # proof MUST fail. This mode exists so the CI job can show that the
        # check goes red instead of quietly skipping.
        Write-Step 'red-path self-check: the application is deliberately NOT started'
    }
    else {
        # --- launch it ------------------------------------------------------
        # [FACT] A child process inherits the environment it was started
        # with, so the token set above reaches the application.
        #
        # The executable inside the bundle is started directly rather than
        # through `open`: `open` would hide the process's own output
        # behind the launch services, and this proof has to be able to
        # report the EXACT error when the application refuses to start.
        Write-Step "starting '$appExec'; waiting for the webview-boot marker"
        $appProcess = Start-Process -FilePath $appExec -PassThru -NoNewWindow `
            -RedirectStandardOutput $appStdout -RedirectStandardError $appStderr
        Write-Host "    started (pid $($appProcess.Id))"
    }

    # --- did the webview report a boot? --------------------------------------
    $markerFile = $null
    $markerDeadline = (Get-Date).AddSeconds($MarkerTimeoutSeconds)
    while ((Get-Date) -lt $markerDeadline) {
        foreach ($dir in $logDirCandidates) {
            $markerFile = Find-BootMarkerInLogs -LogDir $dir -MarkerText $UiBootMarkerText
            if ($null -ne $markerFile) { break }
        }
        if ($null -ne $markerFile) { break }
        if ($null -ne $appProcess -and $appProcess.HasExited) { break }
        Start-Sleep -Milliseconds 500
    }

    if ($null -eq $markerFile) {
        # The failure message has to say what was EXPECTED and what was
        # FOUND, and it must not be swallowed by the cleanup path - the
        # Windows script had exactly that bug, where a real failure was
        # masked by a null-reference error thrown from the `finally` block.
        $expected = $UiBootMarkerText
        $found = @()
        foreach ($dir in $logDirCandidates) {
            if (Test-Path -LiteralPath $dir) {
                $files = @(Get-ChildItem -LiteralPath $dir -File -ErrorAction SilentlyContinue |
                    ForEach-Object { $_.Name }) -join ', '
                $found += "  - $dir (exists; holds: $files)"
            }
            else {
                $found += "  - $dir (does not exist)"
            }
        }
        $appState = if ($null -eq $appProcess) {
            'the application was not started'
        }
        elseif ($appProcess.HasExited) {
            "the application exited with code $($appProcess.ExitCode)"
        }
        else {
            "the application is still running (pid $($appProcess.Id))"
        }
        $appOut = (Get-Content -LiteralPath $appStdout -Raw -ErrorAction SilentlyContinue)
        $appErr = (Get-Content -LiteralPath $appStderr -Raw -ErrorAction SilentlyContinue)
        $why = if ($SelfTestRedPath) {
            'the application was deliberately not started, so no marker with this launch''s ' +
            'token can exist. That is the red path this self-check exists to demonstrate.'
        }
        else {
            "$appState. A window that renders nothing still has a window handle, so this is the " +
            'check that tells a booted application from a blank one.'
        }
        throw ("the webview never reported that the UI booted - no line containing '$expected' " +
            "appeared in the application's log directory. $why`n" +
            "the application's log directories, and what each of them holds:`n" +
            ($found -join "`n") + "`n" +
            "the application's stdout:`n$appOut`n" +
            "the application's stderr:`n$appErr")
    }

    Write-Ok "the webview reported that the UI booted: '$markerFile'"
    Write-Ok "log directory: $(Split-Path -Parent $markerFile)"
    if ($null -ne $appProcess -and -not $appProcess.HasExited) {
        Write-Note "the application is still running (pid $($appProcess.Id)); it is stopped below"
    }
}
catch {
    $exitCode = if ($setupError) { 2 } else { 1 }
    Write-Bad "LAUNCH PROOF FAILED: $($_.Exception.Message)"
}
finally {
    # --- stop it, and unmount the image --------------------------------------
    # Every statement in here is guarded: nothing that happens during the
    # cleanup may change the verdict, because a cleanup error that
    # replaces the real failure is how a red job turns into a green one
    # with the wrong reason.
    if ($null -ne $appProcess) {
        try {
            if (-not $appProcess.HasExited) {
                Write-Note "stopping pid $($appProcess.Id)"
                $appProcess.Kill()
                $appProcess.WaitForExit(10000) | Out-Null
            }
        }
        catch {
            Write-Note "the application could not be stopped: $($_.Exception.Message)"
        }
    }
    if ($mounted) {
        try {
            & hdiutil detach $mountDir 1> $hdiStdout 2> $hdiStderr | Out-Null
            if ($LASTEXITCODE -ne 0) {
                Write-Note "hdiutil detach exited $LASTEXITCODE; trying again with -force"
                & hdiutil detach -force $mountDir 1> $hdiStdout 2> $hdiStderr | Out-Null
            }
            Write-Ok 'unmounted the disk image'
        }
        catch {
            Write-Note "the disk image could not be unmounted: $($_.Exception.Message)"
        }
    }
    try {
        Remove-Item -LiteralPath $work -Recurse -Force -ErrorAction SilentlyContinue
    }
    catch {
        Write-Note "the scratch directory '$work' could not be removed: $($_.Exception.Message)"
    }
}

if ($exitCode -eq 0) {
    Write-Ok 'PASS: the packaged application was launched from the mounted .dmg and its webview reported a boot'
}
exit $exitCode
