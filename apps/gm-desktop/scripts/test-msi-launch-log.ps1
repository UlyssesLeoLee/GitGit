#requires -Version 7
<#
.SYNOPSIS
    Exercise the decision half of the packaged-app launch proof with
    synthetic log files.

.DESCRIPTION
    WHY THIS TEST EXISTS
    ====================
    A gate that has never been shown to fail is not known to be a gate. The
    packaged-app launch proof is the only check in this repository that
    says "the installed application really showed the product", and its
    whole value rests on one question: does the log evidence belong to THIS
    launch?

    The proof that it does used to be positional - the log file's length
    before the launch, and a search of the bytes after it. That design had
    a hole that only opens in the field: `tauri-plugin-log` rotates its
    file by DELETING it and creating a new one, so the recorded length
    points past the end of the new file and a healthy launch is reported
    as a failure.

    The mechanism now is a one-shot token: only the application that was
    given this launch's token can write a line that matches. These cases
    are the proof that the search really does reject the evidence it has to
    reject - a marker left by an earlier run, and a marker written without
    a token at all.

    [FACT] These cases need no MSI, no Windows desktop and no application:
    they exercise `msi-launch-log.ps1`, which is where the decision lives.
    That is the point of splitting the file in two, and it is why this test
    can run in the always-running `gate` workflow on every pull request.

.NOTES
    Exit codes
        0  every case behaved as specified.
        1  at least one case did not; each failure is printed.
#>
[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
if (Get-Variable -Name PSNativeCommandUseErrorActionPreference -ErrorAction SilentlyContinue) {
    $PSNativeCommandUseErrorActionPreference = $false
}

$helper = Join-Path $PSScriptRoot 'msi-launch-log.ps1'
if (-not (Test-Path -LiteralPath $helper)) {
    Write-Error "the launch-proof helper '$helper' is not there"
    exit 2
}
. $helper

$script:Failures = 0
$script:CaseCount = 0

function Write-Case { param([string]$M) Write-Host "    --  $M" -ForegroundColor DarkGray }
function Write-Ok   { param([string]$M) Write-Host "    ok  $M" -ForegroundColor Green }
function Write-Bad  { param([string]$M) Write-Host "    FAIL  $M" -ForegroundColor Red; $script:Failures++ }

function Assert-True {
    param([bool]$Condition, [string]$Because)
    $script:CaseCount++
    if ($Condition) { Write-Ok $Because } else { Write-Bad $Because }
}

function Assert-Equal {
    param($Expected, $Actual, [string]$Because)
    $script:CaseCount++
    if ($Expected -eq $Actual) {
        Write-Ok $Because
    } else {
        Write-Bad ("{0} (expected '{1}', got '{2}')" -f $Because, $Expected, $Actual)
    }
}

# --- a scratch directory that cleans itself up -------------------------------
$scratch = Join-Path ([System.IO.Path]::GetTempPath()) ('gitgit-boot-marker-' + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $scratch -Force | Out-Null

try {
    $marker = 'GITGIT_UI_BOOT_MARKER_V1'
    $nonce = New-BootNonce

    # === 1. the token is a token =========================================
    Write-Case 'a one-shot token is minted per launch'
    $second = New-BootNonce
    Assert-True ($nonce -ne $second) 'two launches get different tokens'
    Assert-True ($nonce -match '^[0-9a-f]{32}$') 'the token is 32 hexadecimal characters'

    # === 2. the searched text is the marker plus the token ================
    Write-Case 'the searched text carries the token'
    $markerText = Get-BootMarkerText -Marker $marker -Nonce $nonce
    Assert-Equal "$marker $nonce" $markerText 'the searched text is the marker and the token'
    Assert-Equal $marker (Get-BootMarkerText -Marker $marker -Nonce '') 'with no token the text is the bare marker'

    # === 3. a marker from an EARLIER run does not satisfy the gate =======
    Write-Case "a marker left by an earlier run is not evidence"
    $stale = Join-Path $scratch 'stale.log'
    Set-Content -LiteralPath $stale -Value ("2026-10-29_00-00-00[gm_desktop_lib::commands::uiboot][INFO] {0} {1}" -f $marker, (New-BootNonce)) -NoNewline
    Assert-True ($null -eq (Find-BootMarkerInLogs -LogDir $scratch -MarkerText $markerText)) `
        'a stale marker does not satisfy the gate'

    # === 4. a marker with NO token does not satisfy the gate ==============
    Write-Case 'a marker written without this run token is not evidence'
    $bare = Join-Path $scratch 'bare.log'
    Set-Content -LiteralPath $bare -Value ("2026-10-29_00-00-00[gm_desktop_lib::commands::uiboot][INFO] {0}" -f $marker) -NoNewline
    Assert-True ($null -eq (Find-BootMarkerInLogs -LogDir $scratch -MarkerText $markerText)) `
        'a tokenless marker does not satisfy the gate'

    # === 5. this run's marker DOES satisfy the gate =======================
    Write-Case 'this run marker is found'
    $mine = Join-Path $scratch 'gitgit Desktop.log'
    Set-Content -LiteralPath $mine -Value ("2026-10-29_00-00-00[gm_desktop_lib::commands::uiboot][INFO] {0}" -f $markerText) -NoNewline
    $found = Find-BootMarkerInLogs -LogDir $scratch -MarkerText $markerText
    Assert-Equal $mine $found 'the file carrying this run marker is returned'

    # === 6. rotation cannot hide the evidence =============================
    # [FACT] `tauri-plugin-log` rotates by DELETING the file and creating a
    # new one, so after a rotation the file is short again and its old
    # bytes are gone. The token is what makes the search position-free: the
    # evidence is found wherever in the new file it happens to be.
    Write-Case 'a rotated log file still carries the evidence'
    Remove-Item -LiteralPath $mine -Force
    Set-Content -LiteralPath $mine -Value ("2026-10-29_00-00-01[gm_desktop_lib::commands::uiboot][INFO] {0}" -f $markerText) -NoNewline
    $foundAfterRotation = Find-BootMarkerInLogs -LogDir $scratch -MarkerText $markerText
    Assert-Equal $mine $foundAfterRotation 'the evidence is found in the rotated file'

    # === 7. an empty log directory is not evidence ========================
    Write-Case 'an empty log directory is not evidence'
    $empty = Join-Path $scratch 'empty'
    New-Item -ItemType Directory -Path $empty -Force | Out-Null
    Assert-True ($null -eq (Find-BootMarkerInLogs -LogDir $empty -MarkerText $markerText)) `
        'nothing is found in an empty directory'
    Assert-True ($null -eq (Find-BootMarkerInLogs -LogDir (Join-Path $scratch 'does-not-exist') -MarkerText $markerText)) `
        'nothing is found in a directory that does not exist'
}
finally {
    Remove-Item -LiteralPath $scratch -Recurse -Force -ErrorAction SilentlyContinue
}

Write-Host ""
if ($script:Failures -gt 0) {
    Write-Host ("{0} of {1} cases FAILED" -f $script:Failures, $script:CaseCount) -ForegroundColor Red
    exit 1
}
Write-Host ("all {0} cases passed" -f $script:CaseCount) -ForegroundColor Green
exit 0
