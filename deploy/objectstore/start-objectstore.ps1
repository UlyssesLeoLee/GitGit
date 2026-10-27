#requires -Version 7
<#
.SYNOPSIS
    Start this repository's own S3-compatible object store (dev/test fixture).

.DESCRIPTION
    The credential vault stores its credentials in an S3-compatible bucket.
    The server that bucket used to live on was GNU AGPLv3 — a licence this
    product cannot carry, because it consumes the server over HTTP. The
    server is therefore this repository's own: the `gitgit-objectstore`
    binary (`src/bin/gitgit-objectstore.rs` over `src/objectstore/`).

    This script is the whole fixture. There is no container runtime, no
    docker-compose file and no separate S3 client: the binary's
    `--bootstrap-bucket` creates the bucket and enables versioning on it,
    which is exactly what the retired fixture's `mc mb` + `mc version
    enable` pair used to do — in a second container, driven by a client
    this repository did not own.

    `[FACT]` Versioning is load-bearing, not decoration.
    `VaultVersionSummary` records the `x-amz-version-id` the server
    returns on PUT, and a server only returns one for a versioned bucket.
    Leave it off and every stored credential keeps an empty `version_id`.

    `[FACT]` The secret key comes from the environment and only from the
    environment. It is not a parameter and it is never a command-line
    argument: a long-lived secret in the process arguments is visible in
    `ps` output and lands in the shell history. The server has no default
    secret either — it refuses to start without one, and so does this
    script.

    `[FACT]` The access key IS a parameter, because an access key is an
    identifier rather than a secret: it names the client, it does not
    authenticate it.

.PARAMETER Root
    The directory the objects are stored in. Default: `.gitgit/objectstore`
    under the repository root.

.PARAMETER Bind
    The address to listen on. Default: `127.0.0.1:9000` — loopback only,
    so nothing off this machine can reach the bucket.

.PARAMETER AccessKey
    The S3 access key the server verifies signatures against. Default:
    `gitgit-dev`. This is an identifier, not a secret.

.PARAMETER Bucket
    The bucket to create and enable versioning on. Default: `gitgit-vault`.

.EXAMPLE
    $env:GITGIT_OBJECTSTORE_SECRET_KEY = 'a-dev-secret'
    pwsh -NoProfile -File deploy/objectstore/start-objectstore.ps1

.EXAMPLE
    $env:GITGIT_OBJECTSTORE_SECRET_KEY = 'a-dev-secret'
    pwsh -NoProfile -File deploy/objectstore/start-objectstore.ps1 -Bind 127.0.0.1:9100
#>

[CmdletBinding()]
param(
    [string]$Root,
    [string]$Bind = '127.0.0.1:9000',
    [string]$AccessKey = 'gitgit-dev',
    [string]$Bucket = 'gitgit-vault'
)

$ErrorActionPreference = 'Stop'

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path

# ── The secret: environment only ─────────────────────────────────────────────
# `[FACT]` Deliberately not a parameter. A parameter would end up in the
# process arguments of *this* script (visible in `ps`) and in the shell
# history, and the rule the whole repository holds to is that a secret
# never goes there — see `src/ai/mod.rs` and `src/bin/gitgit-objectstore.rs`.
$secretKey = $env:GITGIT_OBJECTSTORE_SECRET_KEY
if ([string]::IsNullOrWhiteSpace($secretKey)) {
    Write-Host ''
    Write-Host 'The object store needs a secret key, and it only comes from the environment.' -ForegroundColor Red
    Write-Host ''
    Write-Host '  $env:GITGIT_OBJECTSTORE_SECRET_KEY = ''<something>''' 
    Write-Host ''
    Write-Host 'There is deliberately no default and no --secret-key flag: a server with a' -ForegroundColor Yellow
    Write-Host 'hard-coded credential is a server with a published credential.' -ForegroundColor Yellow
    exit 1
}

if ([string]::IsNullOrWhiteSpace($Root)) {
    $Root = Join-Path $repoRoot '.gitgit/objectstore'
}

# ── Locate or build the binary ───────────────────────────────────────────────
$exe = if ($IsWindows -or $env:OS -eq 'Windows_NT') { '.exe' } else { '' }

function Get-ObjectStoreBinary {
    param([string]$RepoRoot, [string]$Exe)

    $candidates = @()
    if ($env:CARGO_TARGET_DIR) {
        $candidates += (Join-Path $env:CARGO_TARGET_DIR "debug/gitgit-objectstore$Exe")
    }
    $candidates += (Join-Path $RepoRoot 'target/debug/gitgit-objectstore' + $Exe)
    $candidates += (Join-Path $RepoRoot 'target-regression/debug/gitgit-objectstore' + $Exe)
    foreach ($c in $candidates) {
        if ($c -and (Test-Path -LiteralPath $c)) { return $c }
    }
    return $null
}

$binary = Get-ObjectStoreBinary -RepoRoot $repoRoot -Exe $exe
if (-not $binary) {
    Write-Host '[step] building gitgit-objectstore (cargo build --locked --bin gitgit-objectstore)'
    $cargo = Get-Command cargo -CommandType Application -ErrorAction SilentlyContinue |
             Select-Object -First 1
    if (-not $cargo) {
        throw 'cargo not found on PATH; the object store binary has to be built from this repository.'
    }
    Push-Location $repoRoot
    try {
        & $cargo.Source build --locked --bin gitgit-objectstore
        if ($LASTEXITCODE -ne 0) {
            throw "cargo build --bin gitgit-objectstore exited $LASTEXITCODE"
        }
    } finally {
        Pop-Location
    }
    $binary = Get-ObjectStoreBinary -RepoRoot $repoRoot -Exe $exe
    if (-not $binary) {
        throw 'the object store binary was built but could not be found afterwards'
    }
}

# ── Start it ─────────────────────────────────────────────────────────────────
# `--bootstrap-bucket` creates the bucket and turns versioning on. Both
# steps are idempotent, so the flag is safe on every start.
$psi = [System.Diagnostics.ProcessStartInfo]::new()
$psi.FileName               = $binary
$psi.WorkingDirectory       = $repoRoot
$psi.UseShellExecute        = $false
$psi.RedirectStandardOutput = $true
$psi.RedirectStandardError  = $true
[void]$psi.ArgumentList.Add('--root')
[void]$psi.ArgumentList.Add($Root)
[void]$psi.ArgumentList.Add('--bind')
[void]$psi.ArgumentList.Add($Bind)
[void]$psi.ArgumentList.Add('--access-key')
[void]$psi.ArgumentList.Add($AccessKey)
[void]$psi.ArgumentList.Add('--bootstrap-bucket')
[void]$psi.ArgumentList.Add($Bucket)
# The secret travels in the child's environment, not in its arguments.
$psi.Environment['GITGIT_OBJECTSTORE_SECRET_KEY'] = $secretKey

$proc = [System.Diagnostics.Process]::Start($psi)
if (-not $proc) {
    throw 'the object store process could not be started'
}

# ── Wait until it answers ───────────────────────────────────────────────────
$host = $null
$port = $null
try {
    $parsed = [System.Net.IPEndPoint]::Parse($Bind)
    $host = $parsed.Address
    $port = $parsed.Port
} catch {
    Write-Warn "could not parse the bind address '$Bind'; skipping the readiness probe"
}

if ($host) {
    $ready = $false
    for ($i = 0; $i -lt 60; $i++) {
        Start-Sleep -Milliseconds 250
        if ($proc.HasExited) { break }
        try {
            $client = [System.Net.Sockets.TcpClient]::new()
            $client.Connect($host, $port)
            $client.Dispose()
            $ready = $true
            break
        } catch {
            # not up yet
        }
    }
    if (-not $ready) {
        if (-not $proc.HasExited) { try { $proc.Kill($true) } catch { } }
        throw "the object store never started listening on $Bind"
    }
}

$endpoint = "http://$Bind"
Write-Host ''
Write-Host '  object store : listening' -ForegroundColor Green
Write-Host "  endpoint     : $endpoint"
Write-Host "  bucket       : $Bucket (created, versioning enabled)"
Write-Host "  access key   : $AccessKey"
Write-Host "  root         : $Root"
Write-Host ''
Write-Host '  point the credential vault at it with:'
Write-Host ''
Write-Host "    `$env:GITGIT_VAULT_BACKEND  = 's3'"
Write-Host "    `$env:GITGIT_S3_ENDPOINT    = '$endpoint'"
Write-Host "    `$env:GITGIT_S3_BUCKET     = '$Bucket'"
Write-Host "    `$env:GITGIT_S3_KEY_PREFIX = 'gitgit-vault/'"
Write-Host "    `$env:GITGIT_S3_ACCESS_KEY = '$AccessKey'"
Write-Host "    `$env:GITGIT_S3_SECRET_KEY = '<the value of GITGIT_OBJECTSTORE_SECRET_KEY>'"
Write-Host ''
Write-Host '  stop it with Ctrl-C.' -ForegroundColor Yellow
Write-Host ''

# ── Run in the foreground until interrupted ──────────────────────────────────
try {
    while (-not $proc.HasExited) {
        Start-Sleep -Milliseconds 200
    }
    Write-Host ''
    Write-Host "  the object store exited with code $($proc.ExitCode)" -ForegroundColor Yellow
    exit $proc.ExitCode
} finally {
    if (-not $proc.HasExited) {
        try { $proc.Kill($true) } catch { }
    }
}
