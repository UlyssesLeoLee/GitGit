#requires -Version 7
<#
.SYNOPSIS
    Start this repository's own object store and prove the S3 credential vault against it.

.DESCRIPTION
    The credential vault's S3 path used to need a third-party server, a
    container runtime and a separate S3 client before a single test could
    run. The server is now this repository's own
    (`gitgit-objectstore`), and the two vault end-to-end tests start it
    themselves — so this script verifies the deployment the way an
    operator would use it, rather than the way a test would.

    What it proves, in order:
      1. The object store starts and provisions the bucket with versioning
         enabled (`--bootstrap-bucket`; there is no `mc` and no
         docker-compose any more).
      2. The bucket is not anonymously readable: an unsigned list request
         is answered `403`, because every request has to be SigV4-signed.
      3. The **product binary** round-trips a credential through the
         `s3` backend: `gitgit key set` / `get` / `list-keys` /
         `versions` / `delete`, with the backend selected by
         `GITGIT_VAULT_BACKEND=s3`. This is the whole path — the backend
         switch, the SigV4 signing, the server, and the read-back.
      4. BOTH vault end-to-end tests really execute and really pass, in
         an ordinary `cargo test` run — without `--ignored`.

    Two things this script deliberately does not claim:
      * It does not run the long-lived server. It starts one for the
        duration of the check and stops it again.
      * It does not check the acceptance criterion literally as it was
        written. `curl http://localhost:9000/gitgit-vault?list` cannot
        return a bucket list without SigV4 signing, so this script
        asserts the honest form of that probe: an UNSIGNED list request
        must come back 403, which proves the bucket exists and is not
        public. A 200 there would be the actual defect.

    Fail-closed note: a `cargo test` name filter that matches nothing
    exits 0. This script counts the reported passes and fails when the
    expected tests did not run, so a renamed test or a wrong target
    cannot turn this into a green no-op.

.PARAMETER SkipRustTests
    Bring the object store up and assert its state, but skip the two
    Rust end-to-end tests. Useful on a host without a Rust toolchain.

.EXAMPLE
    pwsh -NoProfile -File scripts/verify-s3-vault.ps1
#>

[CmdletBinding()]
param(
    [switch]$SkipRustTests
)

$ErrorActionPreference = 'Stop'

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
. (Join-Path $PSScriptRoot 'lib/regression-common.ps1')

$failures = @()
function Add-Failure($m) { $script:failures += $m }
function Check($cond, $what) { if (-not $cond) { Add-Failure $what } }

# ── tools ─────────────────────────────────────────────────────────────────────
$cargo = Get-ToolPath -Name 'cargo'
if (-not $cargo) {
    throw 'cargo not found on PATH. This check builds the object store and the product binary from this repository.'
}
$curl = Get-ToolPath -Name 'curl'

# ── build the two binaries ────────────────────────────────────────────────────
Write-Step 'building gitgit and gitgit-objectstore'
$build = Invoke-External -FilePath $cargo -WorkingDirectory $repoRoot -TimeoutSec 1800 `
    -ArgumentList @('build', '--locked', '--bin', 'gitgit', '--bin', 'gitgit-objectstore')
if ($build.ExitCode -ne 0) {
    Write-Host ($build.StdErr + $build.StdOut)
    throw "cargo build exited $($build.ExitCode)"
}

# Cargo puts the binaries under $CARGO_TARGET_DIR when it is set, and under
# <repo>/target otherwise. Mirror that rule rather than guessing.
$targetDir = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { Join-Path $repoRoot 'target' }
$exeSuffix = if (Test-IsWindows) { '.exe' } else { '' }
$objectStoreBin = Join-Path $targetDir "debug/gitgit-objectstore$exeSuffix"
$gitgitBin      = Join-Path $targetDir "debug/gitgit$exeSuffix"
foreach ($b in @($objectStoreBin, $gitgitBin)) {
    if (-not (Test-Path -LiteralPath $b)) {
        throw "the built binary was not found at $b"
    }
}

# ── a free port and a throwaway store ─────────────────────────────────────────
function Get-FreeTcpPort {
    # Port 0 lets the OS pick, so a parallel run cannot collide with this
    # one. The listener is closed before the server binds it — a small
    # race, and the reason the readiness probe below is a retry loop and
    # not a single connect.
    $listener = [System.Net.Sockets.TcpListener]::new([System.Net.IPAddress]::Loopback, 0)
    $listener.Start()
    try { return $listener.LocalEndpoint.Port } finally { $listener.Stop() }
}

$storeRoot = Join-Path (Get-ScratchTempRoot) ("gitgit-objectstore-verify-" + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $storeRoot -Force | Out-Null

$port       = Get-FreeTcpPort
$bind       = "127.0.0.1:$port"
$endpoint   = "http://$bind"
$bucket     = 'gitgit-vault'
$accessKey  = 'gitgit-verify-access'
# A fresh secret for every run: this is a throwaway store, and a secret
# that outlives the run is a secret that can outlive its owner.
$secretKey  = [Convert]::ToBase64String([System.Security.Cryptography.RandomNumberGenerator]::GetBytes(32))

Write-Step "starting the object store on $bind (root $storeRoot)"

# ── 1. start it ───────────────────────────────────────────────────────────────
$psi = [System.Diagnostics.ProcessStartInfo]::new()
$psi.FileName               = $objectStoreBin
$psi.WorkingDirectory       = $repoRoot
$psi.UseShellExecute        = $false
$psi.RedirectStandardOutput = $true
$psi.RedirectStandardError  = $true
[void]$psi.ArgumentList.Add('--root')
[void]$psi.ArgumentList.Add($storeRoot)
[void]$psi.ArgumentList.Add('--bind')
[void]$psi.ArgumentList.Add($bind)
[void]$psi.ArgumentList.Add('--access-key')
[void]$psi.ArgumentList.Add($accessKey)
[void]$psi.ArgumentList.Add('--bootstrap-bucket')
[void]$psi.ArgumentList.Add($bucket)
# The secret travels in the child's environment, never in its arguments:
# a long-lived secret in the process arguments is visible in `ps` output
# and lands in the shell history.
$psi.Environment['GITGIT_OBJECTSTORE_SECRET_KEY'] = $secretKey

$server = [System.Diagnostics.Process]::Start($psi)
if (-not $server) {
    Remove-ScratchRoot -Path $storeRoot
    throw 'the object store process could not be started'
}

$serverLog = Join-Path $storeRoot 'objectstore.log'
$exitCode = 1
try {
    # ── 2. wait until it answers ─────────────────────────────────────────────
    $ready = $false
    for ($i = 0; $i -lt 80; $i++) {
        Start-Sleep -Milliseconds 250
        if ($server.HasExited) { break }
        try {
            $client = [System.Net.Sockets.TcpClient]::new()
            $client.Connect([System.Net.IPAddress]::Loopback, $port)
            $client.Dispose()
            $ready = $true
            break
        } catch {
            # not up yet
        }
    }
    if (-not $ready) {
        Add-Failure "the object store never started listening on $bind"
    } else {
        Write-Step "the object store is listening on $endpoint"
    }

    # ── 3. the bucket is private: an unsigned list is 403 ────────────────────
    if ($curl) {
        Write-Step 'unsigned S3 list request (expect 403, not 200)'
        $list = Invoke-External -FilePath $curl -ArgumentList @(
            '-sS', '-o', (Get-NullDevicePath), '-w', '%{http_code}',
            '--max-time', '5', "$endpoint/$bucket`?list"
        )
        $code = $list.StdOut.Trim()
        Write-Host "  HTTP $code from an unsigned GET /$bucket?list"
        Check ($code -eq '403') "unsigned list returned HTTP $code; 403 is the expected answer (bucket exists and is private)"
    } else {
        Add-Failure 'curl not found on PATH; cannot run the unsigned-list probe'
    }

    # ── 4. the product binary, end to end ─────────────────────────────────────
    if ($ready) {
        Write-Step 'the product binary against the s3 backend'
        # The vault's own settings, spelled the way the product spells them.
        # The credentials come from the environment, like every other
        # credential in this repository.
        $env:GITGIT_VAULT_BACKEND  = 's3'
        $env:GITGIT_S3_ENDPOINT    = $endpoint
        $env:GITGIT_S3_BUCKET      = $bucket
        $env:GITGIT_S3_KEY_PREFIX  = 'gitgit-vault/'
        $env:GITGIT_S3_ACCESS_KEY  = $accessKey
        $env:GITGIT_S3_SECRET_KEY  = $secretKey

        $fileVaultRoot = Join-Path $storeRoot 'file-vault'
        $vaultArgs = @(
            '--vault-file-root', $fileVaultRoot,
            '--vault-backend', 's3',
            '--s3-endpoint', $endpoint,
            '--s3-bucket', $bucket,
            '--s3-key-prefix', 'gitgit-vault/'
        )
        $secretValue = 'sk-verify-secret-value'

        $set = Invoke-External -FilePath $gitgitBin -WorkingDirectory $repoRoot -TimeoutSec 120 `
            -ArgumentList ($vaultArgs + @('key', 'set', 'openai', $secretValue))
        Check ($set.ExitCode -eq 0) "gitgit key set exited $($set.ExitCode): $($set.StdErr)"
        Check ($set.StdOut -match 'set openai -> v1') "gitgit key set printed '$($set.StdOut.Trim())'"

        $get = Invoke-External -FilePath $gitgitBin -WorkingDirectory $repoRoot -TimeoutSec 120 `
            -ArgumentList ($vaultArgs + @('key', 'get', 'openai'))
        Check ($get.ExitCode -eq 0) "gitgit key get exited $($get.ExitCode): $($get.StdErr)"
        Check ($get.StdOut.Trim() -eq $secretValue) "gitgit key get returned '$($get.StdOut.Trim())', expected the value that was stored"

        $listKeys = Invoke-External -FilePath $gitgitBin -WorkingDirectory $repoRoot -TimeoutSec 120 `
            -ArgumentList ($vaultArgs + @('key', 'list-keys'))
        Check ($listKeys.ExitCode -eq 0) "gitgit key list-keys exited $($listKeys.ExitCode): $($listKeys.StdErr)"
        Check ($listKeys.StdOut -match '(?m)^openai$') "gitgit key list-keys did not list openai: $($listKeys.StdOut.Trim())"

        # The version timeline has to exist too: the credential's history
        # is the reason the bucket is versioned in the first place.
        $versions = Invoke-External -FilePath $gitgitBin -WorkingDirectory $repoRoot -TimeoutSec 120 `
            -ArgumentList ($vaultArgs + @('key', 'versions', 'openai'))
        Check ($versions.ExitCode -eq 0) "gitgit key versions exited $($versions.ExitCode): $($versions.StdErr)"
        Check ($versions.StdOut -match '(?m)^v1\b') "gitgit key versions printed no v1 entry: $($versions.StdOut.Trim())"

        $delete = Invoke-External -FilePath $gitgitBin -WorkingDirectory $repoRoot -TimeoutSec 120 `
            -ArgumentList ($vaultArgs + @('key', 'delete', 'openai'))
        Check ($delete.ExitCode -eq 0) "gitgit key delete exited $($delete.ExitCode): $($delete.StdErr)"

        $getAfter = Invoke-External -FilePath $gitgitBin -WorkingDirectory $repoRoot -TimeoutSec 120 `
            -ArgumentList ($vaultArgs + @('key', 'get', 'openai'))
        Check ($getAfter.ExitCode -eq 0) "gitgit key get (after delete) exited $($getAfter.ExitCode): $($getAfter.StdErr)"
        Check ($getAfter.StdOut.Trim() -eq '(not set)') "gitgit key get after delete returned '$($getAfter.StdOut.Trim())', expected '(not set)'"

        Write-Host '  --- gitgit key roundtrip ---'
        foreach ($line in ($set.StdOut + $get.StdOut + $listKeys.StdOut + $versions.StdOut + $delete.StdOut + $getAfter.StdOut -split "`n")) {
            if ($line.Trim()) { Write-Host "  $($line.Trim())" }
        }
    }

    # ── 5. the two Rust end-to-end tests ──────────────────────────────────────
    if ($SkipRustTests) {
        Write-Warn 'skipping the two vault end-to-end Rust tests (-SkipRustTests)'
    } else {
        Write-Step 'running the vault end-to-end tests (no --ignored: they start their own server)'
        # `--lib` is load-bearing, not a style choice. `src/lib.rs` owns
        # `pub mod server;` and `src/main.rs` declares no modules, so both
        # tests live in the library target. A `cargo test --bin gitgit`
        # would compile the thin binary, match zero tests, and exit 0 —
        # which is why the pass count is asserted below.
        $t = Invoke-External -FilePath $cargo -WorkingDirectory $repoRoot -TimeoutSec 1800 -ArgumentList @(
            'test', '--locked', '--lib', '--', 'e2e_roundtrip', '--nocapture'
        )
        $tOut = $t.StdOut + "`n" + $t.StdErr

        $passed = 0
        if ($tOut -match 'test result: ok\. (\d+) passed') { $passed = [int]$Matches[1] }

        Check ($t.ExitCode -eq 0) "cargo test --lib -- e2e_roundtrip exited $($t.ExitCode)"
        Check ($tOut -match 's3_vault_e2e_roundtrip \.\.\. ok')            's3_vault_e2e_roundtrip did not run (or did not pass)'
        Check ($tOut -match 's3_vault_versioned_e2e_roundtrip \.\.\. ok') 's3_vault_versioned_e2e_roundtrip did not run (or did not pass)'
        Check ($passed -ge 2) "expected 2 vault e2e tests to pass, libtest reported $passed -- a filter matching nothing exits 0 and would look green"

        Write-Host "  libtest reported: $passed passed"
        foreach ($line in ($tOut -split "`n")) {
            if ($line -match 's3_vault|test result:') { Write-Host "  $line" }
        }
        if ($t.ExitCode -ne 0) {
            Write-Host '  --- cargo output (tail) ---'
            ($tOut -split "`n" | Select-Object -Last 40) | ForEach-Object { Write-Host "  $_" }
        }
    }

    $exitCode = 0
    if ($failures.Count -gt 0) {
        $exitCode = 1
    }
}
finally {
    if ($server -and -not $server.HasExited) {
        try { $server.Kill($true) } catch { }
    }
    # The server's own log, kept next to the store it wrote, for a failed run.
    try {
        if ($server) {
            $server.WaitForExit(5000) | Out-Null
            @($server.StandardOutput.ReadToEnd(), $server.StandardError.ReadToEnd()) |
                Where-Object { $_ } | Set-Content -Path $serverLog -Encoding utf8
        }
    } catch { }
    if ($exitCode -eq 0) {
        Remove-ScratchRoot -Path $storeRoot
    } else {
        Write-Host "  the object store's data directory was kept for inspection: $storeRoot" -ForegroundColor Yellow
    }
}

Write-Host ''
Write-Host ("  endpoint : {0}" -f $endpoint)
Write-Host ("  bucket   : {0}" -f $bucket)
Write-Host ("  store    : {0}" -f $storeRoot)

if ($failures.Count -gt 0) {
    Write-Host ''
    foreach ($f in $failures) { Write-Host "  FAIL: $f" -ForegroundColor Red }
    exit 1
}

Write-Host ''
if ($SkipRustTests) {
    # Never print a claim the run did not earn. Saying "tests passed" in a
    # run that skipped them is the exact failure mode this script exists to
    # prevent, one level up.
    Write-Host '  PASS (fixture only): the object store started, the bucket is versioned and private, the product binary round-tripped a credential' -ForegroundColor Green
    Write-Host '        the two vault end-to-end Rust tests were SKIPPED, not passed' -ForegroundColor Yellow
} else {
    Write-Host '  PASS: the object store started, the bucket is versioned and private, the product binary round-tripped a credential, and both vault end-to-end tests ran and passed' -ForegroundColor Green
}
exit 0
