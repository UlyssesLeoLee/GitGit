#requires -Version 7
<#
.SYNOPSIS
    Authenticode-sign the built .msi files, prove the signature is on them, and
    refresh `manifest.json` so its recorded hashes describe the SIGNED files.

.DESCRIPTION
    Four steps, each of which can fail the script. Nothing is skipped silently.

    1. LOCATE `signtool.exe`
       It is not on PATH on every runner. PATH is tried first, then the Windows
       Kits `bin\<version>\x64` directories, newest version first. If it cannot
       be found that is a FAILURE, not a reason to report success: a signing
       step that quietly does nothing is worse than no signing step, because it
       is believed.

    2. READ THE CERTIFICATE FROM THE ENVIRONMENT
       `GITGIT_MSI_PFX_BASE64` (base64 of a .pfx) and `GITGIT_MSI_PFX_PASSWORD`.

       Base64 rather than a raw blob because GitHub strips trailing newlines
       from secret values, and a .pfx whose last bytes were eaten will fail to
       import with an error that names nothing useful.

       The pfx is imported into `Cert:\CurrentUser\My` and signing is then done
       by thumbprint. That is deliberate: `signtool /p <password>` would put
       the passphrase in the process command line, where it is visible to
       anything on the box. This way it never leaves the process.

    3. SIGN, THEN VERIFY
       Each .msi is signed with SHA-256 for both the file digest and the
       signature, and RFC 3161-timestamped. Timestamping is not optional: an
       Authenticode signature without one stops validating when the
       certificate expires, so an installer signed today would stop verifying
       years from now.

       Verification then distinguishes two very different outcomes, and
       conflating them would be wrong in both directions:
         * `NotSigned` / `HashMismatch` - the signature is absent, or does not
           cover this file's contents. A BUILD DEFECT. Fails.
         * `Valid` - verified against a trusted root. The good case.
         * a trust-chain failure (`UnknownError` / `NotTrusted`) - the signature
           is present and internally consistent, but this runner does not trust
           the issuing root. A DISTRIBUTION concern, not a build defect. Warned
           about, with the reason printed, and not fatal - otherwise every CI
           run would go red on a self-signed certificate and the gate would
           simply be switched off.

       Which of those happened is recorded in `manifest.json`, so the release
       says what is actually true rather than what someone typed.

    4. REFRESH `manifest.json`
       Required, and the reason this script exists rather than a bare
       `signtool` call.

       `build-msi.ps1` records the SHA-256 of every .msi it produced. Signing
       CHANGES those bytes. A manifest left as-is would describe installers
       that no longer exist, and `verify-msi.ps1` cross-checks the manifest
       against the files on disk - so the stale manifest would fail the next
       verification step, or worse, be shipped next to artifacts it does not
       describe.

       The hashes are therefore recomputed from the signed files, and a
       `signing` block recording subject, thumbprint, timestamp URL and the
       per-file verification status is added.

.PARAMETER Dir
    Directory holding the .msi files. Default: `msi-out`, relative to this
    script.

.PARAMETER Required
    Fail when no certificate is configured. This is what the release workflow
    passes: a release that ships unsigned installers while believing it signed
    them is the failure this flag exists to prevent.

    Without it, a missing certificate is a printed NOTE and exit 0, which is
    what the per-pull-request CI job needs - secrets are not available to
    pull requests from forks, and requiring them there would make every such
    PR red without adding any safety.

.PARAMETER TimestampUrl
    RFC 3161 timestamp server. Default: DigiCert's public one.
    Override with `GITGIT_MSI_TIMESTAMP_URL`.

.EXAMPLE
    ./sign-msi.ps1
    Signs if credentials are in the environment; prints a NOTE and exits 0 if
    they are not.

.EXAMPLE
    ./sign-msi.ps1 -Required
    The release path. No credentials is a failure.

.NOTES
    Exit codes
        0  every .msi is signed and verified, or signing was skipped and the
           skip was printed.
        1  signing was required and unavailable, signtool was missing, signing
           failed, a file is NotSigned, a signature does not cover its file, or
           the manifest could not be refreshed.

    Why the release job runs this BEFORE verify-msi.ps1
        Order is build -> sign -> verify, deliberately. Verifying the signed
        artifact means the check covers exactly what ships, and it runs once
        rather than twice. `msiexec /a` extraction is not cheap.

    The certificate and its password never appear on a command line, in the
    log, or in `manifest.json`. Only the subject and thumbprint are recorded,
    which is what makes a release verifiable by a third party.

    Certificate store cleanup
        The signing certificate is removed from `Cert:\CurrentUser\My` and the
        temporary .pfx deleted on every exit path, including failure. Measured
        while testing: importing a .pfx whose certificate is ALREADY in the
        store returns that existing entry rather than creating a second one, so
        the removal takes the pre-existing certificate with it.

        On a CI runner that is the right outcome - the job ends and the
        machine is discarded, and leaving a code-signing key behind would be
        the worse outcome. On a developer machine, run this on a throwaway
        profile or re-import the certificate if you need it again.
#>

[CmdletBinding()]
param(
    [string]$Dir,
    [switch]$Required,
    [string]$TimestampUrl = $env:GITGIT_MSI_TIMESTAMP_URL
)

$ErrorActionPreference = 'Stop'
if (Get-Variable -Name PSNativeCommandUseErrorActionPreference -ErrorAction SilentlyContinue) {
    $PSNativeCommandUseErrorActionPreference = $false
}

$AppDir = Split-Path -Parent $PSScriptRoot
if (-not $Dir) { $Dir = Join-Path $AppDir 'msi-out' }
if (-not $TimestampUrl) { $TimestampUrl = 'http://timestamp.digicert.com' }

function Write-Step { param([string]$M) Write-Host "==> $M" -ForegroundColor Cyan }
function Write-Ok   { param([string]$M) Write-Host "    ok  $M" -ForegroundColor Green }
function Write-Note { param([string]$M) Write-Host "    NOTE: $M" -ForegroundColor Yellow }

# ---------------------------------------------------------------------------
# Inputs
# ---------------------------------------------------------------------------
if (-not (Test-Path -LiteralPath $Dir)) { throw "artifact directory not found: $Dir (run build-msi.ps1 first)" }
$msis = @(Get-ChildItem -LiteralPath $Dir -Filter '*.msi' -File | Sort-Object Name)
if ($msis.Count -eq 0) { throw "no .msi found in $Dir" }

Write-Step "sign $([string]::Join(', ', ($msis | ForEach-Object { $_.Name })))"
Write-Host "    dir      : $Dir"
Write-Host "    timestamp: $TimestampUrl"

# ---------------------------------------------------------------------------
# 1. Credentials
# ---------------------------------------------------------------------------
$pfxBase64 = $env:GITGIT_MSI_PFX_BASE64
$pfxPass   = $env:GITGIT_MSI_PFX_PASSWORD

if ([string]::IsNullOrWhiteSpace($pfxBase64)) {
    if ($Required) {
        throw ("no signing certificate is configured. Set the GITGIT_MSI_PFX_BASE64 secret " +
               "(base64 of the .pfx) and GITGIT_MSI_PFX_PASSWORD. This release is required to " +
               "ship signed installers, and shipping unsigned ones while believing they are " +
               "signed is the exact failure this check exists to prevent.")
    }
    Write-Note "GITGIT_MSI_PFX_BASE64 is not set; the .msi files will be left UNSIGNED."
    Write-Note "That is correct for a pull-request build (secrets are not available to forks)."
    Write-Step "PASS: skipped signing (no credentials, -Required not set)"
    exit 0
}
if ([string]::IsNullOrEmpty($pfxPass)) {
    throw "GITGIT_MSI_PFX_BASE64 is set but GITGIT_MSI_PFX_PASSWORD is empty; refusing to guess."
}

# ---------------------------------------------------------------------------
# 2. signtool
# ---------------------------------------------------------------------------
function Resolve-SignTool {
    $onPath = Get-Command 'signtool.exe' -ErrorAction SilentlyContinue
    if ($onPath) { return $onPath.Source }

    $kits = Join-Path ${env:ProgramFiles(x86)} 'Windows Kits\10\bin'
    if (-not (Test-Path -LiteralPath $kits)) { return $null }

    # Newest version first. String sorting would put 10.0.20348.0 ahead of
    # 10.0.26100.0, which works but silently picks an older SDK when both are
    # installed.
    $found = @(Get-ChildItem -LiteralPath $kits -Directory -ErrorAction SilentlyContinue |
               Where-Object { $_.Name -match '^\d+\.\d+\.\d+\.\d+$' } |
               Sort-Object { [version]$_.Name } -Descending)
    foreach ($d in $found) {
        $exe = Join-Path $d.FullName 'x64\signtool.exe'
        if (Test-Path -LiteralPath $exe) { return $exe }
    }
    return $null
}

$signtool = Resolve-SignTool
if (-not $signtool) {
    throw ("signtool.exe was not found on PATH or under the Windows Kits. Signing cannot be " +
           "performed, and reporting success here would mean shipping unsigned installers " +
           "believed to be signed.")
}
Write-Ok "signtool: $signtool"

# ---------------------------------------------------------------------------
# 3. Import the certificate, sign, verify
# ---------------------------------------------------------------------------
$pfkDir = Join-Path ([System.IO.Path]::GetTempPath()) ("gitgit-sign-" + [guid]::NewGuid().ToString('N'))
$pfxPath = Join-Path $pfkDir 'signing.pfx'
$imported = $null

try {
    New-Item -ItemType Directory -Path $pfkDir -Force | Out-Null
    # Write the bytes directly rather than piping decoded text: `FromBase64String`
    # output through a file cmdlet re-encodes and can corrupt a binary blob.
    [System.IO.File]::WriteAllBytes($pfxPath, [System.Convert]::FromBase64String($pfxBase64))

    $secure = ConvertTo-SecureString -String $pfxPass -AsPlainText -Force
    try {
        $certs = @(Import-PfxCertificate -FilePath $pfxPath -CertStoreLocation 'Cert:\CurrentUser\My' -Password $secure -Exportable:$false)
    } catch {
        # The raw message here is a localised Win32 string ("指定文件不是有效的 PFX 文件"
        # on a zh-CN runner, a different sentence on an en-US one), which names
        # neither the secret nor the likely cause. Measured, not assumed: a
        # base64 blob that is not a .pfx lands here.
        throw ("the .pfx in GITGIT_MSI_PFX_BASE64 could not be imported. The secret must be the " +
               "base64 of the .pfx FILE ITSELF (not a certificate, not the .cer), and " +
               "GITGIT_MSI_PFX_PASSWORD must be the password that .pfx was exported with. " +
               "Regenerate with: `[Convert]::ToBase64String([IO.File]::ReadAllBytes('signing.pfx'))`. " +
               "Underlying error: $($_.Exception.Message)")
    }

    # A .pfx may carry a chain; the one that signs is the end-entity with a
    # private key and the code-signing EKU.
    $signer = $certs | Where-Object {
        $_.HasPrivateKey -and
        ($_.EnhancedKeyUsageList.ObjectId -contains '1.3.6.1.5.5.7.3.3')
    } | Select-Object -First 1
    if (-not $signer) {
        $signer = $certs | Where-Object { $_.HasPrivateKey } | Select-Object -First 1
        if ($signer) {
            Write-Note "no certificate with the code-signing EKU (1.3.6.1.5.5.7.3.3) was found; signing with '$($signer.Subject)' anyway"
        }
    }
    if (-not $signer) { throw "the .pfx imported but carries no certificate with a private key" }

    $thumb = $signer.Thumbprint
    $imported = $thumb
    Write-Ok "certificate: $($signer.Subject)"
    Write-Host "    thumbprint: $thumb"
    Write-Host "    notAfter  : $($signer.NotAfter.ToUniversalTime().ToString('o'))"

    if ($signer.NotAfter -lt (Get-Date)) {
        throw "the signing certificate expired on $($signer.NotAfter.ToUniversalTime().ToString('o'))"
    }
    if ($signer.NotAfter -lt (Get-Date).AddDays(14)) {
        Write-Note ("the certificate expires in {0:N0} days. A release built after that cannot be " +
                    "signed, and installers signed now will stop verifying once the signing " +
                    "timestamp passes expiry unless the certificate is renewed.") -f ($signer.NotAfter - (Get-Date)).TotalDays
    }

    $results = @()
    foreach ($msi in $msis) {
        Write-Host "    signing $($msi.Name) ..."
        $out = & $signtool sign /fd SHA256 /td SHA256 /tr $TimestampUrl /sha1 $thumb $msi.FullName 2>&1
        $code = $LASTEXITCODE
        if ($code -ne 0) {
            throw ("signtool sign failed with exit code $code for '$($msi.Name)':`n" +
                   ($out | Out-String))
        }

        # Verify what was just written, rather than trusting the exit code of
        # the command that wrote it.
        $sig = Get-AuthenticodeSignature -LiteralPath $msi.FullName
        $status = [string]$sig.Status

        if ($status -eq 'NotSigned') {
            throw "'$($msi.Name)' reports NotSigned after a successful signtool run; the artifact would ship unsigned"
        }
        if ($status -eq 'HashMismatch') {
            throw ("'$($msi.Name)' is signed but the signature does not cover this file's contents " +
                   "(HashMismatch). The artifact was modified after signing.")
        }

        $trusted = ($status -eq 'Valid')
        if ($trusted) {
            Write-Ok "signed and verified: $($msi.Name)  [$status]"
        } else {
            Write-Note "signed, but not trusted on this runner: $($msi.Name)  [$status] $($sig.StatusMessage)"
        }

        $results += [pscustomobject]@{
            file        = $msi.Name
            bytes       = [long]$msi.Length
            sha256      = (Get-FileHash -LiteralPath $msi.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
            status      = $status
            trusted     = $trusted
            statusMessage = [string]$sig.StatusMessage
        }
    }

    $allTrusted = @($results | Where-Object { $_.trusted }).Count -eq $results.Count

    # -----------------------------------------------------------------------
    # 4. Refresh manifest.json
    # -----------------------------------------------------------------------
    $manifestPath = Join-Path $Dir 'manifest.json'
    if (Test-Path -LiteralPath $manifestPath) {
        $manifest = Get-Content -LiteralPath $manifestPath -Raw -Encoding UTF8 | ConvertFrom-Json
        foreach ($v in $manifest.variants) {
            $hit = $results | Where-Object { $_.file -eq $v.file }
            if (-not $hit) {
                throw "manifest lists '$($v.file)' but that file was not among the .msi signed in $Dir"
            }
            # The whole point: the recorded hash described the UNSIGNED bytes.
            $v.bytes = $hit.bytes
            $v.sha256 = $hit.sha256
        }
        $manifest | Add-Member -NotePropertyName signing -NotePropertyValue ([pscustomobject]@{
            signed        = $true
            subject       = $signer.Subject
            thumbprint    = $thumb
            notAfter      = $signer.NotAfter.ToUniversalTime().ToString('o')
            timestampUrl  = $TimestampUrl
            allTrusted    = $allTrusted
            variants      = @($results | ForEach-Object { [pscustomobject]@{
                                    file = $_.file; status = $_.status; trusted = $_.trusted } })
        }) -Force
        $manifest.generatedAtUtc = [DateTime]::UtcNow.ToString('o')
        $manifest | ConvertTo-Json -Depth 6 |
            Set-Content -LiteralPath $manifestPath -Encoding UTF8
        Write-Ok "manifest.json refreshed: $($results.Count) variant(s) re-hashed against the SIGNED files"
    } else {
        Write-Note "no manifest.json in $Dir; nothing to refresh"
    }

    Write-Step "PASS: $($results.Count) msi signed, timestamped, verified"
    if (-not $allTrusted) {
        Write-Note "some signatures do not chain to a root this runner trusts. See the per-file status above."
    }
    exit 0
}
finally {
    # Never leave the key material lying around: remove the certificate from
    # the user store and delete the .pfx, on every path including failure.
    if ($imported) {
        Remove-Item -LiteralPath "Cert:\CurrentUser\My\$imported" -Force -ErrorAction SilentlyContinue
    }
    if ($pfkDir -and (Test-Path -LiteralPath $pfkDir)) {
        Remove-Item -LiteralPath $pfkDir -Recurse -Force -ErrorAction SilentlyContinue
    }
}