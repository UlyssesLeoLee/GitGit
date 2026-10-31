#requires -Version 7
<#
.SYNOPSIS
    The dot-sourceable half of the packaged-app launch proof: the one-shot
    token, and the search for the boot marker that carries it.

.DESCRIPTION
    WHY THIS IS A SEPARATE FILE
    ===========================
    `verify-msi-launch.ps1` needs a packaged MSI and a Windows desktop: it
    installs a package, launches the installed application and waits for it.
    That makes it a good gate and a terrible unit test — nothing about the
    logic that decides "is the boot marker in the log" can be exercised
    without a machine that can build and install the product.

    So the decision itself lives here, in a file with no side effects, and
    `verify-msi-launch.ps1` dot-sources it. `test-msi-launch-log.ps1`
    exercises THIS file with synthetic log files, and the always-running
    `gate` workflow runs that test on every pull request. The gate that
    proves the product works and the gate that proves the proof can fail
    are then two different files, and the second one runs everywhere.

    THE ONE-SHOT TOKEN, AND WHY IT REPLACED AN OFFSET
    =================================================
    The application's log file lives in the user's profile and SURVIVES a
    launch, so a marker written by an EARLIER run can be sitting in it. A
    gate that greps for a constant marker can therefore be satisfied by
    history: it would pass without this launch having booted anything.

    The obvious fix is positional - record the file's length before the
    launch and only look at the bytes after it. It was the first
    implementation, and it has a hole that only shows up in the field:
    `tauri-plugin-log` rotates its file at `max_file_size` (1 MiB in this
    application) by DELETING it and creating a fresh one, and it does that
    at START-UP when the existing file is already at the limit. The
    recorded length then points past the end of the new, nearly empty file,
    the marker is never searched for, and a perfectly healthy launch is
    reported as a failure.

    A per-run token removes the whole class of problem instead of patching
    one instance of it: only the process that was given THIS token can
    write THIS line, so a stale marker cannot match, a rotated file cannot
    hide the line, and the search does not have to know where in the file
    the evidence lives. The token travels to the application through the
    environment, which the launcher hands to the child it starts.

.NOTES
    Every function here is side-effect free apart from writing to the
    application's own log directory, and all of them work on any platform
    PowerShell runs on.
#>

# The environment variable that carries the one-shot token from the launcher
# to the application it starts.
#
# [FACT] The launcher sets it in its own process before it starts the
# application, and a child process inherits the environment it was started
# with. The application reads it once, at boot.
$script:BootNonceVariable = 'GITGIT_UI_BOOT_NONCE'

<#
.SYNOPSIS
    Mint the one-shot token for this launch.

.DESCRIPTION
    [FACT] 32 hexadecimal characters: a GUID rendered without its dashes.
    It is long enough that no two launches can collide, and it contains
    nothing that a log formatter would rewrite.
#>
function New-BootNonce {
    return ([guid]::NewGuid().ToString('N'))
}

<#
.SYNOPSIS
    The exact text the application's log line has to contain.

.DESCRIPTION
    The marker constant is the same for every launch; the token is what
    makes this launch's line different from every earlier launch's line.
    Without a token the text is the bare marker, which is what the
    application writes when it is started by anything other than this
    gate.
#>
function Get-BootMarkerText {
    param(
        [Parameter(Mandatory)][string]$Marker,
        [string]$Nonce
    )
    if ([string]::IsNullOrWhiteSpace($Nonce)) {
        return $Marker
    }
    return "$Marker $Nonce"
}

<#
.SYNOPSIS
    Does this one log file contain the marker line?

.DESCRIPTION
    [FACT] The file is opened with FileShare.ReadWrite, because the running
    application holds it open for append, and a reader that demands
    exclusive access would fail on exactly the file it is looking for.

    [FACT] The WHOLE file is searched. That is safe here precisely because
    the searched text carries this launch's token: the file may hold lines
    from earlier launches, and none of them can match.
#>
function Test-LogContainsMarker {
    param(
        [Parameter(Mandatory)][string]$Path,
        [Parameter(Mandatory)][string]$MarkerText
    )
    $stream = $null
    $reader = $null
    try {
        $stream = [System.IO.File]::Open($Path, [System.IO.FileMode]::Open,
            [System.IO.FileAccess]::Read, [System.IO.FileShare]::ReadWrite)
        $reader = [System.IO.StreamReader]::new($stream)
        return $reader.ReadToEnd().Contains($MarkerText)
    } catch {
        # A file that cannot be read is a file that does not carry the
        # evidence. Failing closed here means the caller reports "not
        # found" instead of crashing halfway through the polling loop.
        return $false
    } finally {
        if ($null -ne $reader) { $reader.Dispose() }
        elseif ($null -ne $stream) { $stream.Dispose() }
    }
}

<#
.SYNOPSIS
    Find the log file that carries this launch's boot marker.

.DESCRIPTION
    Returns the full path of the file that contains the marker line, or
    $null when no file in the directory carries it.

    [FACT] Every `*.log` file in the directory is examined, and the ones
    the application has rotated away are not: with the `KeepOne` rotation
    strategy the plugin DELETES the old file instead of archiving it, so a
    rotated-away file holds nothing that could match this launch's token
    anyway.
#>
function Find-BootMarkerInLogs {
    param(
        [Parameter(Mandatory)][string]$LogDir,
        [Parameter(Mandatory)][string]$MarkerText
    )
    if (-not (Test-Path -LiteralPath $LogDir)) { return $null }
    $logs = @(Get-ChildItem -LiteralPath $LogDir -Filter '*.log' -File -ErrorAction SilentlyContinue)
    foreach ($log in $logs) {
        if (Test-LogContainsMarker -Path $log.FullName -MarkerText $MarkerText) {
            return $log.FullName
        }
    }
    return $null
}
