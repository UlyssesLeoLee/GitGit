#requires -Version 7
<#
.SYNOPSIS
    IT (Integration Test) regression tier for the gitgit MVP.

.DESCRIPTION
    Boots a real `gitgit serve` instance on a free port, waits for
    /api/health, runs a fixed set of HTTP assertions against /api/* via
    curl + jq, then tears the server down.

    These are distinct from ULYS-123's in-process axum oneshot tests
    (api.rs tests): they cross a real TCP socket and the full router
    stack (auth middleware, error mapping, JSON serialization,
    path-extractor behavior). They also exercise endpoints a unit test
    cannot easily simulate: 400 vs 404 routing, content-type handling,
    JSON shape contracts end-to-end.

    Baseline expectations come from `scripts/regression-baseline.json`
    (`it.endpoints`). Each baseline entry has expectStatus and one or
    more expectJsonContains / expectJsonShape / expectArrayLenEq /
    minArrayLen / expectBodyContains assertions.

    jq is required for the shape / array-length assertions. Without it
    those three assertion kinds are silently dropped, so the IT tier
    exits 2 (setup error) rather than reporting a weaker pass. The CI
    job installs jq for exactly this reason.

    Exit codes: 0 pass / 1 any assertion failed / 2 setup error.

.PARAMETER SkipBuild
    If set, do not run `cargo build` first. Useful when CI has already
    produced the binary.

.PARAMETER Bind
    Override the bind address (default 127.0.0.1:18099).

.PARAMETER AdminPass
    The password the server under test is started with, via
    GITGIT_ADMIN_PASS. Defaults to a fixed test value because every
    request in the baseline has to authenticate with the same one, and a
    random value would have to be threaded through the baseline file for
    no additional coverage. `[FACT]` Until 2026-10-05 this script sent
    no credentials at all, because `/api/*` had no auth layer to satisfy;
    every baseline entry now answers 401 without `-u`.
#>

[CmdletBinding()]
param(
    [switch]$SkipBuild,
    [string]$Bind,
    # URL-safe: it is handed to curl's `-u` and never percent-encoded.
    [string]$AdminUser = 'admin',
    [string]$AdminPass = 'it-admin-pass'
)

$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'lib/regression-common.ps1')

# Per-worktree target dir to avoid shared-cache lock contention with
# other Multica workspaces (see ULYS-100 memory note). The user-env
# CARGO_TARGET_DIR=E:\DevCache\cargo\target is overridden for this
# script so cargo build doesn't block on sibling workspace locks.
$script:RegressionTargetDir = Join-Path (Get-RepoRoot) 'target-regression'
$env:CARGO_TARGET_DIR = $script:RegressionTargetDir
if (-not (Test-Path $script:RegressionTargetDir)) {
    New-Item -ItemType Directory -Path $script:RegressionTargetDir -Force | Out-Null
}

$baselinePath = Join-Path (Get-RepoRoot) 'scripts/regression-baseline.json'
if (-not (Test-Path $baselinePath)) {
    Write-Fail "baseline not found at $baselinePath"
    exit 2
}
$baseline = Get-Content -Raw $baselinePath | ConvertFrom-Json

# Resolve external tools by name so the same code works on Windows
# (cargo.exe/git.exe/curl.exe) and Linux (cargo/git/curl).
$jq   = Get-ToolPath -Name 'jq'
$curl = Get-ToolPath -Name 'curl'
if (-not $curl) {
    Write-Fail 'curl not found on PATH'
    exit 2
}
# jq gates three assertion kinds (expectJsonShape, expectArrayLenEq,
# minArrayLen). Degrading to substring matching when it is missing would
# quietly shrink the gate — the tier would report a pass it never actually
# earned — so its absence is a setup error, not a warning.
if (-not $jq) {
    Write-Fail 'jq not found on PATH (required for the JSON shape / array-length assertions)'
    exit 2
}

$run = New-RegressionRun -Tier 'it'

# ── Locate or build the binary ─────────────────────────────────────────────
$bin = Resolve-Binary
if (-not $bin) {
    if ($SkipBuild) {
        Write-Fail "binary not found and -SkipBuild was set"
        $run.SetupError = $true
        $report = Write-RegressionReport -Run $run -OutFile (Join-Path (Get-LogRoot) 'it-setup-error.json')
        exit 2
    }
    Write-Step "cargo build (debug, into $script:RegressionTargetDir)"
    $cargo = Get-ToolPath -Name 'cargo'
    if (-not $cargo) {
        Write-Fail 'cargo not found on PATH'
        $run.SetupError = $true
        $report = Write-RegressionReport -Run $run -OutFile (Join-Path (Get-LogRoot) 'it-build-missing-cargo.json')
        exit 2
    }
    # `--locked` keeps CI honest: it fails the build if the lockfile would
    # need to change, so a dependency bump cannot slip in unnoticed. No
    # `--offline`: a fresh CI runner has an empty registry cache and would
    # fail before downloading anything.
    $build = Invoke-External -FilePath $cargo -TimeoutSec 600 `
        -WorkingDirectory (Get-RepoRoot) `
        -ArgumentList @('build', '--locked', '--color=never')
    Set-Content -Path (Join-Path (Get-LogRoot) 'it-build.log') -Value ($build.StdOut + $build.StdErr) -Encoding utf8
    if ($build.TimedOut) {
        Write-Fail 'cargo build timed out (600s)'
        $run.SetupError = $true
        $report = Write-RegressionReport -Run $run -OutFile (Join-Path (Get-LogRoot) 'it-build-timeout.json')
        exit 2
    }
    if ($build.ExitCode -ne 0) {
        Write-Fail "cargo build exit=$($build.ExitCode)"
        ($build.StdErr -split "`n" | Select-Object -Last 20) | ForEach-Object { Write-Host "  $_" }
        $run.SetupError = $true
        $report = Write-RegressionReport -Run $run -OutFile (Join-Path (Get-LogRoot) 'it-build-failed.json')
        exit 2
    }
    $bin = Resolve-Binary
    if (-not $bin) {
        Write-Fail "binary still not found after build"
        $run.SetupError = $true
        exit 2
    }
}
Write-Step "binary: $bin"

# ── Scratch dirs + bind address ─────────────────────────────────────────────
$scratch = New-ScratchRoot -Prefix 'gitgit-it'
$reposDir = Join-Path $scratch 'repos'
$vaultDir = Join-Path $scratch 'vault'
New-Item -ItemType Directory -Path $reposDir -Force | Out-Null
New-Item -ItemType Directory -Path $vaultDir -Force | Out-Null

if (-not $Bind) {
    $Bind = '127.0.0.1:18099'
}
$baseUrl = "http://$Bind"

# ── Seed repos ──────────────────────────────────────────────────────────────
Write-Step "seeding bare repos in $reposDir"
foreach ($name in @('alpha','beta')) {
    $seedOut = & $bin init-repo $name --repos-dir $reposDir 2>&1
    if ($LASTEXITCODE -ne 0) {
        Write-Fail "init-repo $name failed: $($seedOut -join "`n")"
        Remove-ScratchRoot $scratch
        $run.SetupError = $true
        exit 2
    }
}

# ── Start the server ────────────────────────────────────────────────────────
$logFile = Join-Path $scratch 'server.log'
$serverProc = $null
try {
    Write-Step "starting server on $Bind (log: $logFile)"
    $serverArgs = @('serve','--bind',$Bind,'--repos-dir',$reposDir,'--vault-file-root',$vaultDir)
    # `[FACT]` `/api/*` requires HTTP Basic auth (2026-10-05). Every
    # baseline entry below therefore has to authenticate, and the server
    # has to be told which password to expect. The previous value is
    # saved and restored rather than merely unset, because this script
    # may run inside a developer's shell that already exports one.
    $prevAdminPass = $env:GITGIT_ADMIN_PASS
    $prevAdminUser = $env:GITGIT_ADMIN_USER
    $env:GITGIT_ADMIN_PASS = $AdminPass
    $env:GITGIT_ADMIN_USER = $AdminUser
    try {
        $serverProc = Start-Process -FilePath $bin `
            -ArgumentList $serverArgs `
            -NoNewWindow -PassThru `
            -RedirectStandardOutput $logFile `
            -RedirectStandardError "$logFile.err"
    }
    finally {
        if ($null -eq $prevAdminPass) { Remove-Item Env:GITGIT_ADMIN_PASS -ErrorAction SilentlyContinue }
        else { $env:GITGIT_ADMIN_PASS = $prevAdminPass }
        if ($null -eq $prevAdminUser) { Remove-Item Env:GITGIT_ADMIN_USER -ErrorAction SilentlyContinue }
        else { $env:GITGIT_ADMIN_USER = $prevAdminUser }
    }

    $ready = Wait-HttpReady -Url "$baseUrl/api/health"
    if (-not $ready) {
        Write-Fail "server did not become ready"
        Get-Content $logFile -Tail 30 | ForEach-Object { Write-Host "  $_" }
        throw 'server not ready'
    }
    Write-Step "server ready"

    # ── The auth layer itself, before the baseline ──────────────────────────
    #
    # `[FACT]` Everything below this point now sends `-u`, so every
    # baseline assertion would pass identically whether or not the server
    # checked the credential at all. Without this block the IT tier would
    # be, after 2026-10-05, a test suite that cannot detect the auth layer
    # being deleted — the same way the in-process tests are what catch it
    # today, except this one crosses a real socket and the real middleware
    # stack.
    #
    # Three facts are pinned, because each can be broken independently:
    #   1. no credentials            -> 401
    #   2. wrong credentials         -> 401 (not 403, not a silent pass)
    #   3. `GET /api/health`          -> 200, deliberately outside the
    #      layer, so a liveness probe keeps working when the password is
    #      the thing that is wrong.
    Write-Step "auth layer: anonymous / wrong credential / health"

    $anonBody = Join-Path $scratch 'anon.json'
    $wrongBody = Join-Path $scratch 'wrong.json'
    $healthBody = Join-Path $scratch 'health.json'

    function Invoke-Status([string] $outFile, [string[]] $extraArgs, [string] $url) {
        $args = @('--noproxy','*','--silent','--show-error','-o',$outFile,'-w','%{http_code}','--max-time','15')
        $args += $extraArgs
        $args += $url
        $r = Invoke-External -FilePath $curl -ArgumentList $args
        $text = if (Test-Path $outFile) { Get-Content -Raw $outFile } else { '' }
        return @{ Code = $r.StdOut.Trim(); Body = $text }
    }

    $anon = Invoke-Status $anonBody @() "$baseUrl/api/repos"
    Assert-StatusCode -Run $run -Name 'it.auth.anon.repos.status' `
        -Expected 401 -Actual ([int]$anon.Code) `
        -Detail "an unauthenticated GET /api/repos must be refused; the auth layer is not being applied"
    Assert-True -Run $run -Name 'it.auth.anon.repos.bodyContains(unauthenticated)' `
        -Condition $anon.Body.Contains('unauthenticated') `
        -Detail "body first 200 chars: $($anon.Body.Substring(0, [Math]::Min(200, $anon.Body.Length)))"

    $wrong = Invoke-Status $wrongBody @('-u', ('{0}:definitely-not-the-password' -f $AdminUser)) "$baseUrl/api/repos"
    Assert-StatusCode -Run $run -Name 'it.auth.wrong.repos.status' `
        -Expected 401 -Actual ([int]$wrong.Code) `
        -Detail "a wrong password must be refused, not treated as anonymous-but-allowed"

    $health = Invoke-Status $healthBody @() "$baseUrl/api/health"
    Assert-StatusCode -Run $run -Name 'it.auth.health.status' `
        -Expected 200 -Actual ([int]$health.Code) `
        -Detail '/api/health is deliberately outside the auth layer; a liveness probe that needs the password cannot report that the server is up when the password is the problem'

    # ── Run each baseline endpoint ──────────────────────────────────────────
    foreach ($ep in $baseline.it.endpoints) {
        $name = "it.$($ep.method).$($ep.path)"
        if ($ep.PSObject.Properties.Name -contains 'body' -and $ep.body) {
            $name += '.with-body'
        }
        $uri = $baseUrl + $ep.path

        # Invoke curl through Invoke-External, which hands each argv
        # element to the OS verbatim via ProcessStartInfo.ArgumentList.
        # The previous implementation built a `cmd.exe /c "...curl..."`
        # string, which was Windows-only and required hand-rolled
        # escaping for the JSON request bodies; this path has no shell
        # in it, so a body like {"value":"x"} and a header containing a
        # space survive intact on both Windows and Linux.
        $bodyPath = Join-Path $scratch 'body.json'
        if (Test-Path $bodyPath) { Remove-Item $bodyPath -Force }

        $curlArgs = @(
            '--noproxy', '*', '--silent', '--show-error',
            '-o', $bodyPath,
            '-w', '%{http_code}',
            '--max-time', '15',
            # `[FACT]` `-u` rather than a header built by hand: curl
            # base64s `user:password` into `Authorization: Basic`, which
            # is the only encoding the server accepts, and it does so
            # identically on Windows and Linux. A hand-built header would
            # have to reimplement the same encoding in PowerShell.
            '-u', ('{0}:{1}' -f $AdminUser, $AdminPass)
        )
        if ($ep.method -in @('POST','PUT','DELETE')) {
            $curlArgs += @('-X', $ep.method)
        }
        if ($ep.PSObject.Properties.Name -contains 'body' -and $ep.body) {
            $bodyFile = Join-Path $scratch ('req-body-' + ($name -replace '[\\\[\]:\?\*\/]', '_') + '.json')
            Set-Content -Path $bodyFile -Value $ep.body -Encoding ascii -Force
            $curlArgs += @('-H', 'content-type: application/json', '--data-binary', "@$bodyFile")
        }
        $curlArgs += $uri

        $curlRun = Invoke-External -FilePath $curl -ArgumentList $curlArgs
        $exit = $curlRun.ExitCode
        $stdoutText = $curlRun.StdOut
        $body = if (Test-Path $bodyPath) { Get-Content -Raw $bodyPath } else { '' }

        # curl exits 0 on 4xx/5xx (those are valid HTTP responses).
        # We only fail out if curl itself errored (network/timeout).
        if ($exit -ne 0) {
            Assert-True -Run $run -Name $name -Condition $false `
                -Detail "curl exited $exit; stdout=[$stdoutText]"
            continue
        }

        # The `-w "%{http_code}"` template writes the http code as
        # the ONLY line on stdout (no body — body goes to body.json).
        $statusStr = ($stdoutText.Trim() -split "`n" | Where-Object { $_.Trim() } | Select-Object -Last 1).Trim()
        $statusCode = 0
        if (-not [int]::TryParse($statusStr, [ref]$statusCode)) {
            Assert-True -Run $run -Name $name -Condition $false `
                -Detail "could not parse status '$statusStr' body=[$($body.Substring(0, [Math]::Min(200, $body.Length)))]"
            continue
        }

        Assert-StatusCode -Run $run -Name "$name.status" `
            -Expected ([int]$ep.expectStatus) -Actual $statusCode

        if ($ep.PSObject.Properties.Name -contains 'expectJsonContains') {
            foreach ($needle in $ep.expectJsonContains) {
                $found = $false
                $m = [regex]::Match($needle, '^"([^"]+)":\s*(.+?)\s*$')
                if ($m.Success) {
                    $k = $m.Groups[1].Value
                    $v = $m.Groups[2].Value
                    $jqFilter = if ($v -match '^".*"$') { ".${k} == ${v}" }
                                elseif ($v -match '^-?\d+(\.\d+)?$') { ".${k} == ${v}" }
                                elseif ($v -in @('true','false','null')) { ".${k} == ${v}" }
                                else { $null }
                    if ($jqFilter) {
                        # jq can't read a file passed as an argv
                        # element via PowerShell's Start-Process
                        # either (same quote-stripping issue).
                        # Write the filter to a temp file and
                        # invoke via `-f`.
                        $jqPath = Join-Path (Get-ScratchTempRoot) ("gitgit-regression-it-filter-{0}.jq" -f ([guid]::NewGuid().ToString('N')))
                        Set-Content -Path $jqPath -Value $jqFilter -Encoding ascii -Force
                        try {
                            $jqOut = & $jq -e -f $jqPath $bodyPath 2>$null
                            $found = ($LASTEXITCODE -eq 0)
                        } finally {
                            Remove-Item $jqPath -Force -ErrorAction SilentlyContinue
                        }
                    } else {
                        $found = $body.Contains($needle)
                    }
                } else {
                    $found = $body.Contains($needle)
                }
                Assert-True -Run $run -Name "$name.contains($needle)" -Condition $found `
                    -Detail "needle=$needle body=$(($body -replace "`n",' ') | Select-Object -First 200)"
            }
        }

        if ($ep.PSObject.Properties.Name -contains 'expectJsonShape') {
            $shape = $ep.expectJsonShape
            $matched = $false
            $jqType = if ($shape -eq 'array') { 'array' } elseif ($shape -eq 'object') { 'object' } else { $shape }
            $shapeFilter = 'type == "' + $jqType + '"'
            $shapePath = Join-Path (Get-ScratchTempRoot) ("gitgit-regression-it-shape-{0}.jq" -f ([guid]::NewGuid().ToString('N')))
            Set-Content -Path $shapePath -Value $shapeFilter -Encoding ascii -Force
            try {
                $jqOut = & $jq -e -f $shapePath $bodyPath 2>$null
                $matched = ($LASTEXITCODE -eq 0)
            } finally {
                Remove-Item $shapePath -Force -ErrorAction SilentlyContinue
            }
            Assert-True -Run $run -Name "$name.shape=$shape" -Condition $matched `
                -Detail "expected JSON $shape"
        }

        if ($ep.PSObject.Properties.Name -contains 'expectArrayLenEq') {
            $expectedLen = [int]$ep.expectArrayLenEq
            $actualLen = & $jq length $bodyPath 2>$null
            Assert-Equal -Run $run -Name "$name.arrayLen" `
                -Expected $expectedLen -Actual ([int]$actualLen)
        }

        if ($ep.PSObject.Properties.Name -contains 'minArrayLen') {
            $minLen = [int]$ep.minArrayLen
            $actualLen = [int](& $jq length $bodyPath 2>$null)
            Assert-True -Run $run -Name "$name.arrayLen>=$minLen" `
                -Condition ($actualLen -ge $minLen) `
                -Detail "actual=$actualLen"
        }

        if ($ep.PSObject.Properties.Name -contains 'expectBodyContains') {
            foreach ($needle in $ep.expectBodyContains) {
                Assert-True -Run $run -Name "$name.bodyContains($needle)" `
                    -Condition $body.Contains($needle) `
                    -Detail "body first 200 chars: $($body.Substring(0, [Math]::Min(200, $body.Length)))"
            }
        }
    }
}
finally {
    if ($serverProc -and -not $serverProc.HasExited) {
        Write-Step "stopping server (pid=$($serverProc.Id))"
        Stop-Process -Id $serverProc.Id -Force -ErrorAction SilentlyContinue
        $serverProc.WaitForExit(5000) | Out-Null
    }
    # Preserve scratch for diagnosis on failure, clean on success.
    if ($run.Fail -gt 0) {
        Write-Warn "IT run had failures ($($run.Fail)) — preserving scratch at $scratch for diagnosis"
    } else {
        Remove-ScratchRoot $scratch
    }
}

# ── Report ──────────────────────────────────────────────────────────────────
$outJson = Join-Path (Get-LogRoot) ('it-{0}.json' -f (Get-Date).ToString('yyyyMMdd-HHmmss'))
$report = Write-RegressionReport -Run $run -OutFile $outJson
Write-Host ''
Write-Host '── IT summary ──'
Write-Host ('  regression cases    : pass={0} fail={1} skip={2}' -f $run.Pass, $run.Fail, $run.Skip)
Write-Host ('  baseline endpoints  : {0}' -f $baseline.it.endpoints.Count)
Write-Host ('  json report         : {0}' -f $outJson)
if ($run.SetupError) { exit 2 }
if ($run.Fail -gt 0) { exit 1 }
exit 0