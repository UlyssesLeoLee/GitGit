#requires -Version 7
<#
.SYNOPSIS
    Assert that the MSI signing material is configured, before anything
    expensive is built.

.DESCRIPTION
    WHY THIS SCRIPT EXISTS
    =====================
    [FACT] `apps/gm-desktop/scripts/sign-msi.ps1` already fails closed. With
    `-Required` and no `GITGIT_MSI_PFX_BASE64` in the environment, it throws,
    and the message names both environment variables.

    [FACT] `.github/workflows/release.yml` calls it with `-Required` in the
    `msi` job - but it calls it AFTER `build-msi.ps1` has produced the
    installers.

    [INFERENCE] So a missing certificate is discovered only after a build that
    takes minutes, and by then the reason the run is red has scrolled off the
    top of the log. The check itself is right; where it sits is not.

    This script is that same assertion, moved to the front of the job. It
    reads the same two environment variables and reports the same condition,
    but it runs BEFORE the build, so "the certificate is not there" is the
    first thing the log says rather than the last.

    WHAT IT CHECKS
    ==============
    Two environment variables, read from the environment and never from the
    command line:

      GITGIT_MSI_PFX_BASE64     the base64 of the .pfx file itself
      GITGIT_MSI_PFX_PASSWORD   the password that .pfx was exported with

    A variable that is unset, set to an empty string, or set to nothing but
    whitespace counts as NOT configured. Whitespace is not a certificate, and
    a password of spaces is not a password.

    WHAT IT PRINTS
    ==============
    On success: one line, and nothing else.

    On failure it says the three things a failure has to say to be worth
    anything:
      * which variable is missing, and whether it is unset, empty or blank;
      * how the certificate is delivered to CI at all - the repository secret
        names, the environment variable names they are mapped onto, who sets
        them, and in which contexts they exist;
      * what happens when they are still missing: no release is produced.

    The certificate and the passphrase are never printed - not on success, not
    on failure, not in any part of the message. This script reports WHETHER
    the material is there, never WHAT it is. The values are read from the
    environment and never from an argument, so they cannot reach a command
    line or a log line.

.PARAMETER Required
    State explicitly that the signing material is required.

    [FACT] The release path passes `-Required`, and `sign-msi.ps1` uses the
    same switch for the same reason: a release that ships unsigned installers
    while believing it signed them is the failure these checks exist to
    prevent.

    [FACT] This script has no lenient mode, and that is deliberate.
    `sign-msi.ps1` needs one because it is called from two places - the daily
    `msi` job in `gm-desktop.yml`, where GitHub does not provide secrets to a
    pull request from a fork, and the release path, where it does. This
    script is called from ONE place, the release path, so there is no caller
    a lenient mode would serve. A missing certificate is a failure here
    either way; the switch states the intent at the call site rather than
    changing the verdict.

.EXAMPLE
    ./assert-signing-config.ps1 -Required
    The release path. Exits 1 when the signing material is not configured.

.NOTES
    Exit codes
        0  the signing material is configured.
        1  it is not: at least one of the two variables is unset, empty or
           blank. The message names the variable, how the material is
           delivered to CI, and the consequence.

    Why the release job runs this BEFORE the build
        [INFERENCE] The MSI build takes minutes. Asserting the certificate
        after it means the run is red for a long time before it says why,
        and the reason is the cheapest thing in the pipeline to check. So it
        is checked first, and the build is skipped entirely when the
        material is missing.

    Where the delivery of the certificate is written down
        [FACT] `.github/CI.md`, in the section "Secrets 配置" - the secret
        names, the environment variables they are mapped onto, who sets
        them, when they exist, and what happens when they do not.
#>

[CmdletBinding()]
param(
    [switch]$Required
)

$ErrorActionPreference = 'Stop'
if (Get-Variable -Name PSNativeCommandUseErrorActionPreference -ErrorAction SilentlyContinue) {
    $PSNativeCommandUseErrorActionPreference = $false
}

# [FACT] These are the two environment variables `sign-msi.ps1` reads, and
# the two the release workflow maps its repository secrets onto. They are
# written out here rather than imported from `sign-msi.ps1` so that this
# script stands on its own and can be tested without the other one.
$EnvVarBase64   = 'GITGIT_MSI_PFX_BASE64'
$EnvVarPassword = 'GITGIT_MSI_PFX_PASSWORD'
$SecretBase64   = 'MSI_SIGN_PFX_BASE64'
$SecretPassword = 'MSI_SIGN_PFX_PASSWORD'

function Get-MaterialState {
    param([string]$Name)

    # [FACT] An environment variable that is not set at all reads as $null,
    # and one that is set to an empty value reads as the empty string.
    # PowerShell keeps those two apart, and so does this check: "not set" and
    # "set to nothing" are different mistakes, and a message that says which
    # one it is saves the reader a round trip.
    $value = [System.Environment]::GetEnvironmentVariable($Name)
    if ($null -eq $value) { return 'notset' }
    if ($value -eq '')     { return 'empty' }
    if ([string]::IsNullOrWhiteSpace($value)) { return 'blank' }
    return 'configured'
}

function Describe-MaterialState {
    param([string]$State)

    switch ($State) {
        'notset' { return 'is not set' }
        'empty'  { return 'is set to an empty string' }
        'blank'  { return 'is set but contains only whitespace' }
        default  { return 'is configured' }
    }
}

# ---------------------------------------------------------------------------
# The check
# ---------------------------------------------------------------------------
$material = @(
    [pscustomobject]@{
        Variable = $EnvVarBase64
        State    = (Get-MaterialState $EnvVarBase64)
    }
    [pscustomobject]@{
        Variable = $EnvVarPassword
        State    = (Get-MaterialState $EnvVarPassword)
    }
)

$missing = @($material | Where-Object { $_.State -ne 'configured' })

if ($missing.Count -eq 0) {
    # [FACT] The names are printed, never the values. There is nothing in
    # this line that could be a certificate or a passphrase.
    Write-Host ("    ok  the signing material is configured ({0}, {1})" -f $EnvVarBase64, $EnvVarPassword)
    exit 0
}

# ---------------------------------------------------------------------------
# The report
# ---------------------------------------------------------------------------
# [FACT] The missing line names ONLY the variables that are actually
# missing, so "which half did I forget" is answered by one line rather than
# by reading the whole report.
$missingLine = ($missing | ForEach-Object { "{0} {1}" -f $_.Variable, (Describe-MaterialState $_.State) }) -join '; '

$report = @(
    'the MSI signing material is not configured.'
    ''
    "missing: $missingLine"
    ''
    'how the certificate is delivered to CI:'
    "  repository secret $SecretBase64   -> environment variable $EnvVarBase64"
    "  repository secret $SecretPassword -> environment variable $EnvVarPassword"
    '  both are set by a repository administrator, under Settings > Secrets and variables > Actions > Repository secrets'
    '  they exist only in a context that has repository secrets: the release workflow, which runs on a v* tag or on workflow_dispatch'
    '  a pull request from a fork does not get them, because GitHub does not pass repository secrets to a fork pull request'
    '  that is why the daily msi job in gm-desktop.yml signs only when the material happens to be there, and does not require it'
    ''
    'consequence:'
    '  no release is produced. The msi job of .github/workflows/release.yml fails closed, the draft-release job that would create the draft release never runs, and no draft release exists.'
)

if ($Required) {
    $report += '  this run requires signed installers: the release path signs with -Required, so unsigned installers may not ship.'
}

# [FACT] The report goes to the error stream, because it IS the error, and
# the script exits 1. [FACT] Nothing in it is a certificate or a passphrase:
# the values were read to decide the verdict and are not in the message.
Write-Error -Message ($report -join [System.Environment]::NewLine) -ErrorAction Continue
exit 1
