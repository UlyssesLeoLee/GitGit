#requires -Version 7
<#
.SYNOPSIS
    Bring up the dev/test minIO fixture and prove MinioVault against it.

.DESCRIPTION
    V0 task T11. Until now the repo contained no reproducible deployment
    at all: `git ls-files` had no docker-compose / compose.yml, and the
    two `#[ignore]`d minIO tests in src/server/vault.rs and
    src/server/vault_versioned.rs had never been executed anywhere. This
    script closes that gap and, in the process, replaces two claims that
    turned out to be wrong.

    What it proves, in order:
      1. `deploy/minio/docker-compose.yml` actually starts and the
         bucket is provisioned with versioning enabled.
      2. Versioning is on, which is load-bearing rather than decorative:
         VaultVersionSummary records the `x-amz-version-id` that minIO
         only returns for a versioned bucket.
      3. The bucket is not anonymously readable.
      4. BOTH `#[ignore]`d minIO tests really execute and really pass
         against a live server (set/get/list/rotate/delete, and the
         version-id capture path).

    Two things this script deliberately does not claim:
      * It does not make the product use minIO. `build_vault()` in
        src/main.rs hardwires `FileVault`; there is no CLI switch that
        selects `MinioVault`. This exercises the library.
      * It does not check the T11 acceptance criterion literally as
        written. `curl http://localhost:9000/gitgit-vault?list` cannot
        return a bucket list without SigV4 signing, so this script
        asserts the honest form of that probe: an UNSIGNED list request
        must come back 403, which proves the bucket exists and is not
        public. A 200 there would be the actual defect.

    Fail-closed note: a `cargo test` name filter that matches nothing
    exits 0. This script counts the reported passes and fails when the
    expected tests did not run, so a renamed test or a wrong target
    cannot turn this into a green no-op.

.PARAMETER TearDown
    Stop the fixture and delete its volume (and therefore every stored
    credential) after the run.

.PARAMETER SkipRustTests
    Bring the fixture up and assert its state, but skip the two
    `#[ignore]`d Rust tests. Useful on a host without a Rust toolchain.

.EXAMPLE
    pwsh -NoProfile -File scripts/verify-minio-vault.ps1

.EXAMPLE
    pwsh -NoProfile -File scripts/verify-minio-vault.ps1 -TearDown
#>

[CmdletBinding()]
param(
    [switch]$TearDown,
    [switch]$SkipRustTests
)

$ErrorActionPreference = 'Stop'

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
. (Join-Path $PSScriptRoot 'lib/regression-common.ps1')

$composeFile = Join-Path $repoRoot 'deploy/minio/docker-compose.yml'
if (-not (Test-Path -LiteralPath $composeFile)) {
    throw "compose file not found: $composeFile"
}

# Mirrors the compose defaults. Exported to the child processes below so
# the fixture and the Rust tests cannot drift apart.
$endpoint    = if ($env:GITGIT_MINIO_ENDPOINT)    { $env:GITGIT_MINIO_ENDPOINT }    else { 'http://127.0.0.1:9000' }
$bucket      = if ($env:GITGIT_MINIO_BUCKET)      { $env:GITGIT_MINIO_BUCKET }      else { 'gitgit-vault' }
$accessKey   = if ($env:GITGIT_MINIO_ACCESS_KEY) { $env:GITGIT_MINIO_ACCESS_KEY } else { 'minio' }
$secretKey   = if ($env:GITGIT_MINIO_SECRET_KEY) { $env:GITGIT_MINIO_SECRET_KEY } else { 'minio123' }
$env:GITGIT_MINIO_ENDPOINT    = $endpoint
$env:GITGIT_MINIO_BUCKET      = $bucket
$env:GITGIT_MINIO_ACCESS_KEY  = $accessKey
$env:GITGIT_MINIO_SECRET_KEY  = $secretKey

$failures = @()
function Add-Failure($m) { $script:failures += $m }
function Check($cond, $what) { if (-not $cond) { Add-Failure $what } }

$docker = Get-ToolPath -Name 'docker'
if (-not $docker) {
    throw 'docker not found on PATH. This check needs a container runtime; install Docker Desktop or run it on a host that has one.'
}

function Invoke-Compose([string[]]$composeArgs, [int]$timeoutSec = 900) {
    return Invoke-External -FilePath $docker `
        -ArgumentList (@('compose', '-f', $composeFile) + $composeArgs) `
        -WorkingDirectory $repoRoot -TimeoutSec $timeoutSec
}

Write-Step "fixture: $composeFile"
Write-Step "endpoint $endpoint, bucket $bucket"

# ---------------------------------------------------------------- 1. start
Write-Step 'docker compose up -d'
$up = Invoke-Compose @('up', '-d')
# Not fatal here: a non-zero `up` is usually the one-shot `createbuckets`
# container exiting before minIO was healthy, and the explicit
# provisioning run below produces a far more readable error.
if ($up.ExitCode -ne 0) {
    Write-Warn "docker compose up exited $($up.ExitCode):"
    Write-Host (($up.StdErr + $up.StdOut) -join "`n")
}

# ------------------------------------------------------------- 2. liveness
Write-Step 'waiting for /minio/health/ready'
$ready = Wait-HttpReady -Url "$endpoint/minio/health/ready" -MaxAttempts 60 -DelayMs 500
if (-not $ready) {
    $logs = Invoke-Compose @('logs', '--no-color', '--tail', '40', 'minio')
    Write-Host ($logs.StdOut)
    Add-Failure "minIO never answered $endpoint/minio/health/ready"
}
Check $ready 'minIO answers /minio/health/ready'

if ($failures.Count -gt 0) {
    Write-Host ''
    foreach ($f in $failures) { Write-Host "  FAIL: $f" -ForegroundColor Red }
    Write-Host ''
    Write-Host '  the fixture never came up; the container logs above are the real cause' -ForegroundColor Red
    exit 1
}

# -------------------------------------------------------- 3. provisioning
# Idempotent by construction (`mb --ignore-existing` plus a retry loop),
# so this is safe to run even though `up -d` already ran it.
Write-Step 'running bucket provisioning (idempotent)'
$provision = Invoke-Compose @('run', '--rm', 'createbuckets')
if ($provision.ExitCode -ne 0) {
    Add-Failure "createbuckets exited $($provision.ExitCode)"
    Write-Host (($provision.StdErr + $provision.StdOut) -join "`n")
}

# ------------------------------------------------------------ 4. assertions
$mcScript = @'
set -eu
mc alias set local http://minio:9000 "$GITGIT_MINIO_ACCESS_KEY" "$GITGIT_MINIO_SECRET_KEY" >/dev/null
if mc stat "local/$GITGIT_MINIO_BUCKET" >/dev/null 2>&1; then
  echo "PROBE_BUCKET_EXISTS=yes"
else
  echo "PROBE_BUCKET_EXISTS=no"
fi
echo "PROBE_VERSION_BEGIN"
mc version info "local/$GITGIT_MINIO_BUCKET" 2>&1 || echo "version info failed"
echo "PROBE_VERSION_END"
echo "PROBE_ANON_BEGIN"
mc anonymous get "local/$GITGIT_MINIO_BUCKET" 2>&1 || echo "anonymous get failed"
echo "PROBE_ANON_END"
'@

Write-Step 'probing bucket state through mc'
$mc = Invoke-Compose @('run', '--rm', '--no-TTY', '--entrypoint', '/bin/sh', 'createbuckets', '-c', $mcScript)
$mcText = $mc.StdOut + "`n" + $mc.StdErr

Check ($mcText -match 'PROBE_BUCKET_EXISTS=yes') "bucket '$bucket' does not exist"
# "Versioning: Enabled" is what `mc version info` prints for an enabled
# bucket. Matching the word alone would also accept "Disabled", so the
# negative form is excluded too.
Check ($mcText -match 'Enabled')      'bucket versioning is not enabled (mc version info never said Enabled)'
Check ($mcText -notmatch 'Disabled')  'bucket versioning is reported Disabled'
# `mc anonymous get` prints the *effective* access level, which is
# `private` for a bucket with no anonymous policy. The S3 policy document
# itself spells that `none`. Both are "not anonymously readable", so
# either word is a pass; the check fails only if neither appears, which
# is the case that would actually matter.
Check (($mcText -match '(?m)\bprivate\b') -or ($mcText -match '(?m)\bnone\b')) `
      'bucket is neither private nor policy-none; it may be anonymously readable'

Write-Host '  --- mc version info / anonymous ---'
foreach ($line in ($mcText -split "`n")) {
    if ($line -match 'Versioning|permission|none|Enabled|Disabled') { Write-Host "  $line" }
}

# ------------------------------------------- 5. unsigned list must be 403
# The T11 criterion as written cannot be satisfied literally: listing a
# bucket over the S3 API requires a SigV4 signature. What an unsigned
# request proves is that the bucket exists AND is not public, and the
# honest expected answer is 403.
Write-Step 'unsigned S3 list request (expect 403, not 200)'
$curl = Get-ToolPath -Name 'curl'
if (-not $curl) {
    Add-Failure 'curl not found on PATH; cannot run the unsigned-list probe'
} else {
    $list = Invoke-External -FilePath $curl -ArgumentList @(
        '-sS', '-o', (Get-NullDevicePath), '-w', '%{http_code}',
        '--max-time', '5', "$endpoint/$bucket`?list"
    )
    $code = $list.StdOut.Trim()
    Write-Host "  HTTP $code from an unsigned GET /$bucket?list"
    Check ($code -eq '403') "unsigned list returned HTTP $code; 403 is the expected answer (bucket exists and is private)"
}

# ------------------------------------------------------- 6. the Rust tests
if ($SkipRustTests) {
    Write-Warn 'skipping the two #[ignore]d minIO tests (-SkipRustTests)'
} else {
    $cargo = Get-ToolPath -Name 'cargo'
    if (-not $cargo) {
        Add-Failure 'cargo not found on PATH; cannot run the minIO e2e tests'
    } else {
        # `--lib` is load-bearing, not a style choice. `src/lib.rs` owns
        # `pub mod server;` and `src/main.rs` declares no modules, so both
        # tests live in the library target. The doc comment on
        # minio_vault_e2e_roundtrip says `cargo test --bin gitgit`,
        # which compiles the thin binary, matches zero tests, and exits
        # 0. That is why the pass count is asserted below.
        Write-Step 'running the #[ignore]d minIO tests against the live server'
        $t = Invoke-External -FilePath $cargo -WorkingDirectory $repoRoot -TimeoutSec 1800 -ArgumentList @(
            'test', '--locked', '--lib', '--', '--ignored', 'minio_vault', '--nocapture'
        )
        $tOut = $t.StdOut + "`n" + $t.StdErr

        $passed = 0
        if ($tOut -match 'test result: ok\. (\d+) passed') { $passed = [int]$Matches[1] }

        Check ($t.ExitCode -eq 0) "cargo test --lib -- --ignored minio_vault exited $($t.ExitCode)"
        Check ($tOut -match 'minio_vault_e2e_roundtrip \.\.\. ok')            'minio_vault_e2e_roundtrip did not run (or did not pass)'
        Check ($tOut -match 'minio_vault_versioned_e2e_roundtrip \.\.\. ok') 'minio_vault_versioned_e2e_roundtrip did not run (or did not pass)'
        Check ($passed -ge 2) "expected 2 minIO e2e tests to pass, libtest reported $passed -- a filter matching nothing exits 0 and would look green"

        Write-Host "  libtest reported: $passed passed"
        foreach ($line in ($tOut -split "`n")) {
            if ($line -match 'minio_vault|test result:') { Write-Host "  $line" }
        }
        if ($t.ExitCode -ne 0) {
            Write-Host '  --- cargo output (tail) ---'
            ($tOut -split "`n" | Select-Object -Last 40) | ForEach-Object { Write-Host "  $_" }
        }
    }
}

# ----------------------------------------------------------------- 7. done
Write-Host ''
Write-Host ("  endpoint : {0}" -f $endpoint)
Write-Host ("  bucket   : {0}" -f $bucket)
Write-Host ("  compose  : {0}" -f $composeFile)

if ($TearDown) {
    Write-Step 'tearing the fixture down (-v deletes the volume and every stored credential)'
    $down = Invoke-Compose @('down', '-v')
    Check ($down.ExitCode -eq 0) "docker compose down -v exited $($down.ExitCode)"
}

if ($failures.Count -gt 0) {
    Write-Host ''
    foreach ($f in $failures) { Write-Host "  FAIL: $f" -ForegroundColor Red }
    if (-not $TearDown) {
        Write-Host ''
        Write-Host ("  fixture still running; stop it with: docker compose -f {0} down" -f $composeFile)
    }
    exit 1
}

Write-Host ''
if ($SkipRustTests) {
    # Never print a claim the run did not earn. Saying "tests passed" in a
    # run that skipped them is the exact failure mode this script exists to
    # prevent, one level up.
    Write-Host '  PASS (fixture only): fixture reproducible, bucket provisioned + versioned + private' -ForegroundColor Green
    Write-Host '        the two #[ignore]d minIO e2e tests were SKIPPED, not passed' -ForegroundColor Yellow
} else {
    Write-Host '  PASS: fixture reproducible, bucket provisioned + versioned + private, both #[ignore]d minIO e2e tests ran and passed' -ForegroundColor Green
}
if (-not $TearDown) {
    Write-Host ("  stop it with: docker compose -f {0} down   (add -v to drop stored credentials)" -f $composeFile)
}
exit 0
