#requires -Version 7
<#
.SYNOPSIS
    Exercise `assert-signing-config.ps1` with the inputs that must fail and
    the one that must pass.

.DESCRIPTION
    WHY THIS TEST EXISTS
    ====================
    A gate that has never been shown to fail is not known to be a gate.

    `assert-signing-config.ps1` is what makes a missing MSI certificate fail
    the release job in its FIRST minute rather than after a build that takes
    minutes. That is only worth anything if the script really does fail when
    the material is missing, and really does pass when it is there.

    [FACT] These cases need no MSI, no Windows desktop, no application and no
    certificate: the decision lives entirely in two environment variables.
    That is the point of splitting the check out of `sign-msi.ps1`, and it is
    why this test can run in the always-running `gate` workflow on every pull
    request.

    [FACT] The values used below are SENTINELS. `ZmFrZS1wZmgtZmFrZQ==` is not
    a certificate and `fake-password` is not a passphrase. Nothing is signed
    with them, and nothing tries to import them - the script under test only
    asks whether they are present and non-blank.

    THE CASES
    =========
      1. neither variable set          -> non-zero, and the report says so
      2. only one half set              -> non-zero, and the report names the
                                          half that is missing, not the half
                                          that is configured
      3. both set but blank            -> non-zero, because whitespace is not
                                          a certificate
      4. both set to the empty string  -> non-zero, for the same reason
      5. both configured                -> zero, and neither sentinel appears
                                          in the output

.NOTES
    Exit codes
        0  every case behaved as specified.
        1  at least one case did not; each failure is printed.
        2  the script under test is not there, so nothing could be checked.
#>
[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
if (Get-Variable -Name PSNativeCommandUseErrorActionPreference -ErrorAction SilentlyContinue) {
    $PSNativeCommandUseErrorActionPreference = $false
}

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

$scriptUnderTest = Join-Path $PSScriptRoot 'assert-signing-config.ps1'
if (-not (Test-Path -LiteralPath $scriptUnderTest)) {
    Write-Error "the script under test '$scriptUnderTest' is not there"
    exit 2
}

# [FACT] The sentinels are fake. They are here to be detected in the output
# (or to not be), never to be used as key material.
$FakeBase64   = 'ZmFrZS1wZmgtZmFrZQ=='
$FakePassword = 'fake-password'

# [FACT] The child is the same PowerShell that is running this test, found
# from the process itself rather than from PATH. [INFERENCE] PATH could hold a
# different PowerShell - or none - and the child has to be able to run a
# script that requires version 7.
$pwshPath = [System.Environment]::ProcessPath
if (-not $pwshPath) { $pwshPath = 'pwsh' }

# [FACT] PowerShell writes ANSI colour escapes into the error stream even
# when the output is captured instead of shown on a terminal, so the text is
# stripped before it is matched. The escapes are formatting, not content.
$AnsiPattern = [regex]::Escape([string][char]27) + '\[[0-9;]*[A-Za-z]'

function Get-MissingLine {
    param([string]$Output)

    foreach ($line in ($Output -split "`r?`n")) {
        if ($line -match '^\s*missing:') { return $line }
    }
    return $null
}

function Invoke-Assert {
    param(
        [AllowNull()][string]$Base64,
        [AllowNull()][string]$Password,
        [switch]$Required
    )

    # [FACT] `$env:X = $null` removes the variable from the process
    # environment; `$env:X = ''` sets it to an empty string. PowerShell keeps
    # those two apart, and so does the script under test - which is why the
    # "not set" and the "empty" cases are separate cases here.
    $env:GITGIT_MSI_PFX_BASE64   = $Base64
    $env:GITGIT_MSI_PFX_PASSWORD = $Password

    $argList = @('-NoProfile', '-File', $scriptUnderTest)
    if ($Required) { $argList += '-Required' }

    # [FACT] stdout and stderr are captured together: the report is written
    # to the error stream, because it IS the error.
    $output = & $pwshPath @argList 2>&1 | Out-String
    $code = $LASTEXITCODE

    return [pscustomobject]@{
        Code   = $code
        Output = ($output -replace $AnsiPattern, '')
    }
}

# [FACT] The caller's own environment is restored at the end, so running this
# test does not leave the signing variables set for whatever runs next.
$originalBase64   = [System.Environment]::GetEnvironmentVariable('GITGIT_MSI_PFX_BASE64')
$originalPassword = [System.Environment]::GetEnvironmentVariable('GITGIT_MSI_PFX_PASSWORD')

try {
    # === 1. nothing is configured ==========================================
    Write-Case 'no signing material at all'
    $r = Invoke-Assert -Base64 $null -Password $null -Required
    Assert-True ($r.Code -ne 0) 'the script exits non-zero when neither variable is set'

    $missingLine = Get-MissingLine $r.Output
    Assert-True ($null -ne $missingLine) 'the report has a line that says what is missing'
    Assert-True ($missingLine -like '*GITGIT_MSI_PFX_BASE64*') 'the missing line names GITGIT_MSI_PFX_BASE64'
    Assert-True ($missingLine -like '*GITGIT_MSI_PFX_PASSWORD*') 'the missing line names GITGIT_MSI_PFX_PASSWORD'

    Assert-True ($r.Output -like '*MSI_SIGN_PFX_BASE64*') 'the report names the repository secret MSI_SIGN_PFX_BASE64'
    Assert-True ($r.Output -like '*MSI_SIGN_PFX_PASSWORD*') 'the report names the repository secret MSI_SIGN_PFX_PASSWORD'
    Assert-True ($r.Output -like '*no release is produced*') 'the report says that no release is produced'

    # === 2. only one half is configured =====================================
    Write-Case 'only one half of the material is configured'
    $r = Invoke-Assert -Base64 $FakeBase64 -Password $null -Required
    Assert-True ($r.Code -ne 0) 'the script exits non-zero when only one variable is set'

    $missingLine = Get-MissingLine $r.Output
    Assert-True ($null -ne $missingLine) 'the report has a line that says what is missing'
    Assert-True ($missingLine -like '*GITGIT_MSI_PFX_PASSWORD*') 'the missing line names GITGIT_MSI_PFX_PASSWORD'
    Assert-True (-not ($missingLine -like '*GITGIT_MSI_PFX_BASE64*')) `
        'the missing line does not blame GITGIT_MSI_PFX_BASE64, which IS configured'
    Assert-True ($r.Output -like '*no release is produced*') 'the report says that no release is produced'
    Assert-True (-not ($r.Output -like "*$FakeBase64*")) 'the report does not print the certificate'

    # === 3. configured, but blank ===========================================
    Write-Case 'both variables are set but blank'
    $r = Invoke-Assert -Base64 '   ' -Password "`t" -Required
    Assert-True ($r.Code -ne 0) 'the script exits non-zero when the variables hold only whitespace'

    $missingLine = Get-MissingLine $r.Output
    Assert-True ($null -ne $missingLine) 'the report has a line that says what is missing'
    Assert-True ($missingLine -like '*GITGIT_MSI_PFX_BASE64*') 'the missing line names GITGIT_MSI_PFX_BASE64'
    Assert-True ($missingLine -like '*GITGIT_MSI_PFX_PASSWORD*') 'the missing line names GITGIT_MSI_PFX_PASSWORD'
    Assert-True ($missingLine -match 'whitespace') 'the missing line says the value holds only whitespace'
    Assert-True ($r.Output -like '*no release is produced*') 'the report says that no release is produced'

    # === 4. configured, but empty ===========================================
    Write-Case 'both variables are set to an empty string'
    $r = Invoke-Assert -Base64 '' -Password '' -Required
    Assert-True ($r.Code -ne 0) 'the script exits non-zero when the variables hold an empty string'

    $missingLine = Get-MissingLine $r.Output
    Assert-True ($null -ne $missingLine) 'the report has a line that says what is missing'
    Assert-True ($missingLine -like '*GITGIT_MSI_PFX_BASE64*') 'the missing line names GITGIT_MSI_PFX_BASE64'
    Assert-True ($missingLine -like '*GITGIT_MSI_PFX_PASSWORD*') 'the missing line names GITGIT_MSI_PFX_PASSWORD'
    Assert-True ($missingLine -match 'empty') 'the missing line says the value is an empty string'

    # === 5. the material is there ===========================================
    Write-Case 'the material is configured'
    $r = Invoke-Assert -Base64 $FakeBase64 -Password $FakePassword -Required
    Assert-Equal 0 $r.Code 'the script exits zero when both variables are configured'
    Assert-True ($r.Output -match 'ok') 'the script prints an ok line'
    Assert-True (-not ($r.Output -like "*$FakeBase64*")) 'the output does not contain the certificate'
    Assert-True (-not ($r.Output -like "*$FakePassword*")) 'the output does not contain the passphrase'

    # === 6. the check fails closed without -Required too ====================
    # [FACT] The script has no lenient mode: it is called from the release
    # path only, so a missing certificate is a failure either way.
    Write-Case 'the check fails closed even without -Required'
    $r = Invoke-Assert -Base64 $null -Password $null
    Assert-True ($r.Code -ne 0) 'the script exits non-zero without -Required too'
    Assert-True ($r.Output -like '*no release is produced*') 'the report says that no release is produced'
}
finally {
    $env:GITGIT_MSI_PFX_BASE64   = $originalBase64
    $env:GITGIT_MSI_PFX_PASSWORD = $originalPassword
}

Write-Host ""
if ($script:Failures -gt 0) {
    Write-Host ("{0} of {1} cases FAILED" -f $script:Failures, $script:CaseCount) -ForegroundColor Red
    exit 1
}
Write-Host ("all {0} cases passed" -f $script:CaseCount) -ForegroundColor Green
exit 0
