#requires -Version 5
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

    Exit codes: 0 pass / 1 any assertion failed / 2 setup error.

.PARAMETER SkipBuild
    If set, do not run `cargo build` first. Useful when CI has already
    produced the binary.

.PARAMETER Bind
    Override the bind address (default 127.0.0.1:18099).
#>

[CmdletBinding()]
param(
    [switch]$SkipBuild,
    [string]$Bind
)

$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'lib\regression-common.ps1')

# Per-worktree target dir to avoid shared-cache lock contention with
# other Multica workspaces (see ULYS-100 memory note). The user-env
# CARGO_TARGET_DIR=E:\DevCache\cargo\target is overridden for this
# script so cargo build doesn't block on sibling workspace locks.
$script:RegressionTargetDir = Join-Path (Get-RepoRoot) 'target-regression'
$env:CARGO_TARGET_DIR = $script:RegressionTargetDir
if (-not (Test-Path $script:RegressionTargetDir)) {
    New-Item -ItemType Directory -Path $script:RegressionTargetDir -Force | Out-Null
}

$baselinePath = Join-Path (Get-RepoRoot) 'scripts\regression-baseline.json'
if (-not (Test-Path $baselinePath)) {
    Write-Fail "baseline not found at $baselinePath"
    exit 2
}
$baseline = Get-Content -Raw $baselinePath | ConvertFrom-Json

$jq = (Get-Command jq.exe -ErrorAction SilentlyContinue)
$curl = (Get-Command curl.exe -ErrorAction SilentlyContinue)
if (-not $curl) {
    Write-Fail 'curl not found on PATH'
    exit 2
}
# jq is recommended but optional; the script falls back to grep when missing.
$hasJq = [bool]$jq

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
    # Build into our per-worktree target dir. Use the bash-wrapper
    # pattern as regression-ut.ps1 to dodge the PowerShell
    # Start-Process + bash.exe exit-code-empty bug.
    Write-Step "cargo build (debug, into $script:RegressionTargetDir)"
    $bash = (Get-Command bash.exe -ErrorAction SilentlyContinue)
    if (-not $bash) {
        Write-Fail 'bash.exe not found on PATH (expected from git-bash)'
        $run.SetupError = $true
        exit 2
    }
    $repoRootPosix = (Get-RepoRoot) -replace '\\','/'
    $cargoBinPosix = ((Get-Command cargo.exe).Source) -replace '\\','/'
    $buildLog = Join-Path (Get-LogRoot) 'it-build-stdout.log'
    $buildErr = Join-Path (Get-LogRoot) 'it-build-stderr.log'
    $exitCodePath = Join-Path (Get-LogRoot) 'it-build-exitcode.txt'
    if (Test-Path $buildLog) { Remove-Item $buildLog -Force }
    if (Test-Path $buildErr) { Remove-Item $buildErr -Force }
    if (Test-Path $exitCodePath) { Remove-Item $exitCodePath -Force }
    $shPath = Join-Path $env:TEMP ("gitgit-regression-it-build-{0}.sh" -f ([guid]::NewGuid().ToString('N')))
    $shBody = "#!/usr/bin/env bash`n" +
              "set -u`n" +
              "cd '$repoRootPosix'`n" +
              "'$cargoBinPosix' build --color=never --offline > '$($buildLog -replace '\\','/')' 2> '$($buildErr -replace '\\','/')'`n" +
              "echo `$? > '$($exitCodePath -replace '\\','/')'`n"
    Set-Content -Path $shPath -Value $shBody -Encoding ascii -Force
    try {
        $proc = Start-Process -FilePath $bash.Source `
            -ArgumentList @($shPath) `
            -NoNewWindow `
            -PassThru
        if (-not $proc.WaitForExit(600000)) {
            try { Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue } catch {}
            Write-Fail 'cargo build timed out (600s)'
            $run.SetupError = $true
            $report = Write-RegressionReport -Run $run -OutFile (Join-Path (Get-LogRoot) 'it-build-timeout.json')
            exit 2
        }
    } finally {
        Remove-Item $shPath -Force -ErrorAction SilentlyContinue
    }
    $buildExit = -1
    if (Test-Path $exitCodePath) {
        $exitStr = (Get-Content $exitCodePath -Raw -ErrorAction SilentlyContinue).Trim()
        if ($exitStr -match '^\-?\d+$') { $buildExit = [int]$exitStr }
    }
    if ($buildExit -ne 0) {
        Write-Fail "cargo build exit=$buildExit"
        Get-Content $buildErr -Tail 20 -ErrorAction SilentlyContinue | ForEach-Object { Write-Host "  $_" }
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
    $serverArgs = @('serve','--bind',$Bind,'--repos-dir',$reposDir)
    if ($bin -like '*\target-regression\debug\gitgit.exe') {
        $serverArgs += @('--vault-file-root',$vaultDir)
    }
    $serverProc = Start-Process -FilePath $bin `
        -ArgumentList $serverArgs `
        -NoNewWindow -PassThru `
        -RedirectStandardOutput $logFile `
        -RedirectStandardError "$logFile.err"

    # Wait for /api/health to come up. Probe with curl.
    $ready = Wait-HttpReady -Url "$baseUrl/api/health"
    if (-not $ready) {
        Write-Fail "server did not become ready"
        Get-Content $logFile -Tail 30 | ForEach-Object { Write-Host "  $_" }
        throw 'server not ready'
    }
    Write-Step "server ready"

    # ── Run each baseline endpoint ──────────────────────────────────────────
    foreach ($ep in $baseline.it.endpoints) {
        $name = "it.$($ep.method).$($ep.path)"
        if ($ep.PSObject.Properties.Name -contains 'body' -and $ep.body) {
            $name += '.with-body'
        }
        $uri = $baseUrl + $ep.path
        # `--noproxy *` already tells curl to bypass the env proxy
        # in the inherited environment (HTTPS_PROXY=127.0.0.1:10808
        # in this user's bash, which curl's libcurl picks up by
        # default on Windows).
        $curlArgs = @('--noproxy','*','--silent','--show-error','-o',(Join-Path $scratch 'body.json'),
                      '-w','%{http_code}','--max-time','5')
        if ($ep.method -in @('POST','PUT','DELETE')) {
            $curlArgs += @('-X',$ep.method)
        }
        if ($ep.PSObject.Properties.Name -contains 'body' -and $ep.body) {
            # PowerShell's `& curl.exe @curlArgs` strips `"` chars
            # from each argument during expansion, so JSON bodies
            # like `{"value":"x"}` arrive at curl as `{value:x}` and
            # fail JSON parsing. Backslash-escape every quote in
            # the body before passing it.
            $bodyEscaped = $ep.body -replace '"','\"'
            $curlArgs += @('-H','content-type: application/json','-d', $bodyEscaped)
        }
        $curlArgs += @($uri)

        # Capture stdout (http code from -w) + stderr (progress
        # bars). curl writes the progress meter to stderr even with
        # -sS in some Windows builds; that stderr gets re-classified
        # by PowerShell as a RemoteException which, under
        # ErrorActionPreference=Stop, aborts the script. Suppress
        # errors at the call boundary by reading stdout/stderr into
        # separate buffers. Then take the LAST non-empty line of
        # the merged stream as the http code (curl writes the -w
        # template once, at the end of stdout, so it survives even
        # after progress bars).
        $stdoutFile = Join-Path $scratch 'curl-stdout.txt'
        $stderrFile = Join-Path $scratch 'curl-stderr.txt'
        if (Test-Path $stdoutFile) { Remove-Item $stdoutFile -Force }
        if (Test-Path $stderrFile) { Remove-Item $stderrFile -Force }
        $proc = Start-Process -FilePath curl.exe -ArgumentList $curlArgs `
            -NoNewWindow -PassThru `
            -RedirectStandardOutput $stdoutFile `
            -RedirectStandardError $stderrFile
        $proc.WaitForExit(30000) | Out-Null
        $stdoutText = if (Test-Path $stdoutFile) { Get-Content $stdoutFile -Raw -ErrorAction SilentlyContinue } else { '' }
        $stderrText = if (Test-Path $stderrFile) { Get-Content $stderrFile -Raw -ErrorAction SilentlyContinue } else { '' }
        # curl writes the -w template to stdout (last line), the
        # progress meter to stderr. Status code = LAST non-empty
        # line of stdout (the body, if any, comes before).
        $statusStr = ($stdoutText.Trim() -split "`n" | Where-Object { $_.Trim() } | Select-Object -Last 1).Trim()
        if ($LASTEXITCODE -ne 0) {
            Assert-True -Run $run -Name $name -Condition $false `
                -Detail "curl exited $LASTEXITCODE"
            continue
        }
        $statusCode = 0
        if (-not [int]::TryParse($statusStr, [ref]$statusCode)) {
            Assert-True -Run $run -Name $name -Condition $false `
                -Detail "could not parse status '$statusStr'"
            continue
        }

        Assert-StatusCode -Run $run -Name "$name.status" `
            -Expected ([int]$ep.expectStatus) -Actual $statusCode

        $bodyPath = Join-Path $scratch 'body.json'
        $body = if (Test-Path $bodyPath) { Get-Content -Raw $bodyPath } else { '' }

        if ($ep.PSObject.Properties.Name -contains 'expectJsonContains') {
            foreach ($needle in $ep.expectJsonContains) {
                $found = $false
                if ($hasJq) {
                    $m = [regex]::Match($needle, '^"([^"]+)":\s*(.+?)\s*$')
                    if ($m.Success) {
                        $k = $m.Groups[1].Value
                        $v = $m.Groups[2].Value
                        $jqFilter = if ($v -match '^".*"$') { ".${k} == ${v}" }
                                    elseif ($v -match '^-?\d+(\.\d+)?$') { ".${k} == ${v}" }
                                    elseif ($v -in @('true','false','null')) { ".${k} == ${v}" }
                                    else { $null }
                        if ($jqFilter) {
                            # PowerShell's `& jq.exe @argv` expansion
                            # strips `"` literals from each argument,
                            # so filters like `.status == "ok"`
                            # become `.status == ok` by the time jq
                            # sees them — and jq parses that as a
                            # division. Workaround: write the filter
                            # to a temp .jq file and invoke with
                            # `-f`, which is also how the same pattern
                            # is documented for jq invocations from
                            # shell wrappers.
                            $jqPath = Join-Path $env:TEMP ("gitgit-regression-it-filter-{0}.jq" -f ([guid]::NewGuid().ToString('N')))
                            # Use ASCII encoding (no BOM) — PowerShell's
                            # `utf8` adds a BOM which jq then chokes on.
                            Set-Content -Path $jqPath -Value $jqFilter -Encoding ascii -Force
                            try {
                                $jqOut = & jq.exe -e -f $jqPath $bodyPath 2>$null
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
            if ($hasJq) {
                $jqType = if ($shape -eq 'array') { 'array' } elseif ($shape -eq 'object') { 'object' } else { $shape }
                $shapeFilter = 'type == "' + $jqType + '"'
                $shapePath = Join-Path $env:TEMP ("gitgit-regression-it-shape-{0}.jq" -f ([guid]::NewGuid().ToString('N')))
                Set-Content -Path $shapePath -Value $shapeFilter -Encoding ascii -Force
                try {
                    $jqOut = & jq.exe -e -f $shapePath $bodyPath 2>$null
                    $matched = ($LASTEXITCODE -eq 0)
                } finally {
                    Remove-Item $shapePath -Force -ErrorAction SilentlyContinue
                }
            } else {
                $trim = $body.Trim()
                if ($shape -eq 'array') { $matched = $trim.StartsWith('[') }
                elseif ($shape -eq 'object') { $matched = $trim.StartsWith('{') }
            }
            Assert-True -Run $run -Name "$name.shape=$shape" -Condition $matched `
                -Detail "expected JSON $shape"
        }

        if ($ep.PSObject.Properties.Name -contains 'expectArrayLenEq' -and $hasJq) {
            $expectedLen = [int]$ep.expectArrayLenEq
            $actualLen = & jq.exe length $bodyPath 2>$null
            Assert-Equal -Run $run -Name "$name.arrayLen" `
                -Expected $expectedLen -Actual ([int]$actualLen)
        }

        if ($ep.PSObject.Properties.Name -contains 'minArrayLen' -and $hasJq) {
            $minLen = [int]$ep.minArrayLen
            $actualLen = [int](& jq.exe length $bodyPath 2>$null)
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
if (-not $hasJq) {
    Write-Host ('  note: jq not found; JSON-shape & array-length assertions were skipped') -ForegroundColor Yellow
}
if ($run.SetupError) { exit 2 }
if ($run.Fail -gt 0) { exit 1 }
exit 0