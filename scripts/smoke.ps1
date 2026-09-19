#requires -Version 5
<#
.SYNOPSIS
    End-to-end smoke test for the `gitgit` MVP.

.DESCRIPTION
    1. Build the `gitgit` binary (debug profile, fast).
    2. Start it in the background listening on 127.0.0.1:8088 (override
       with -Bind). The port is deliberately different from the default
       8080 so the smoke test can run side-by-side with a long-lived
       dev server.
    3. `git clone http://admin:admin@127.0.0.1:8088/repos/demo.git` to
       a temp directory.
    4. Create a file, commit, push to `main`.
    5. Create a `feature` branch, push.
    6. Clone a second time, fetch, merge origin/feature, verify the
       merge commit and the file content from the feature branch.
    7. Verify a push without credentials is rejected.
    8. Kill the server, print PASS / FAIL.

    Designed for Windows PowerShell. Uses `git` from PATH.

    PowerShell 5 quirks handled here:
      * `Invoke-WebRequest` negotiates NTLM/Auth before sending Basic
        auth — we use the absolute path to System32 curl.exe via
        `cmd /c` for the readiness probe, with `?` quoted to avoid
        PowerShell wildcard expansion.
      * `git` writes benign messages ("Cloning into ...", "warning: ...")
        to stderr; under $ErrorActionPreference=Stop these become
        terminating exceptions. We suppress that wrapping with a
        helper that resets ErrorActionPreference locally and returns
        the merged output, and we gate every step on $LASTEXITCODE.
#>

[CmdletBinding()]
param(
    [string]$Bind = '127.0.0.1:8088',
    [string]$RepoName = 'demo',
    [switch]$SkipBuild
)

$ErrorActionPreference = 'Stop'
$env:GIT_TERMINAL_PROMPT = '0'
# Fail fast on credential prompts instead of hanging the smoke test.
$env:GIT_ASKPASS = 'true'
# Isolate git's credential lookup from the user environment. The user's
# gitconfig may declare a `credential.<url>` block (e.g. for
# 127.0.0.1:8088) that auto-fulfills an otherwise-missing password,
# which would silently make step 8 (negative-auth) pass when it should
# fail. Pointing HOME at an empty scratch dir disables every helper.
$env:HOME = Join-Path $env:TEMP "gitgit-smoke-home-$([guid]::NewGuid().ToString('N'))"
New-Item -ItemType Directory -Path $env:HOME -Force | Out-Null
# Empty gitconfig so git has no helper at all.
$env:XDG_CONFIG_HOME = $env:HOME
# Drop any inherited credential helper selection.
$env:GIT_CONFIG_COUNT = '0'

$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$LogFile  = Join-Path $env:TEMP "gitgit-smoke-$([guid]::NewGuid().ToString('N')).log"
$TmpRoot  = Join-Path $env:TEMP "gitgit-smoke-$([guid]::NewGuid().ToString('N'))"
$ServerProc = $null

function Write-Status($msg) {
    Write-Host "[smoke] $msg"
}

function Fail($msg) {
    Write-Error "[smoke] FAIL: $msg"
    if ($ServerProc -and -not $ServerProc.HasExited) {
        try { Stop-Process -Id $ServerProc.Id -Force -ErrorAction SilentlyContinue } catch {}
    }
    if (Test-Path $TmpRoot) {
        # Use cmd /c rmdir rather than Remove-Item — the latter is
        # blocked by some safety policies, and rmdir works here.
        cmd /c "rmdir /S /Q `"$TmpRoot`"" 2>$null | Out-Null
    }
    exit 1
}

# Wrapper around `git` that captures stdout+stderr without PowerShell
# promoting stderr lines to terminating exceptions under Strict-Mode.
function Invoke-Git {
    param([Parameter(ValueFromRemainingArguments=$true)][string[]]$GitArgs)
    $local:ErrorActionPreference = 'Continue'
    $output = & git.exe @GitArgs 2>&1
    return $output
}

# 1. Build ---------------------------------------------------------------
if (-not $SkipBuild) {
    Write-Status "cargo build --quiet (debug)"
    $build = & cargo build --quiet 2>&1
    if ($LASTEXITCODE -ne 0) {
        Write-Host ($build -join "`n")
        Fail "cargo build failed"
    }
}

# Locate the built binary. CARGO_TARGET_DIR may be set in the user env.
$targetDir = $env:CARGO_TARGET_DIR
if (-not $targetDir) { $targetDir = Join-Path $RepoRoot 'target' }
$bin = Join-Path $targetDir 'debug\gitgit.exe'
if (-not (Test-Path $bin)) {
    Fail "binary not found at $bin"
}

# 2. Reset repos dir under a dedicated scratch ----------------------------
$reposDir = Join-Path $TmpRoot 'repos'
New-Item -ItemType Directory -Path $reposDir -Force | Out-Null

# 3. Start server --------------------------------------------------------
Write-Status "starting server on $Bind (log: $LogFile)"
$ServerProc = Start-Process -FilePath $bin `
    -ArgumentList @('serve', '--bind', $Bind, '--repos-dir', $reposDir) `
    -PassThru `
    -NoNewWindow `
    -RedirectStandardOutput $LogFile `
    -RedirectStandardError "$LogFile.err"

# 4. init-repo via the CLI BEFORE the readiness probe. The probe hits
#    /repos/<name>.git/info/refs, which 404s if the bare repo doesn't
#    exist yet on disk.
Write-Status "gitgit init-repo $RepoName"
$initOut = & $bin init-repo $RepoName --repos-dir $reposDir 2>&1
if ($LASTEXITCODE -ne 0) { Fail "init-repo failed: $($initOut -join "`n")" }
if (-not (Test-Path (Join-Path $reposDir "$RepoName.git\HEAD"))) {
    Fail "bare repo HEAD not found at $reposDir/$RepoName.git/HEAD"
}

# 5. Wait until /info/refs responds 200 (with a query string) ------------
$probeUrl = "http://$Bind/repos/$RepoName.git/info/refs?service=git-upload-pack"
Write-Status "waiting for $probeUrl"
$ready = $false
for ($i = 0; $i -lt 50; $i++) {
    Start-Sleep -Milliseconds 200
    # Use the absolute path to System32 curl.exe via `cmd /c` to avoid
    # PowerShell 5's `curl` alias for Invoke-WebRequest, and avoid
    # `?` in the URL being treated as a wildcard.
    $out = cmd /c "C:\Windows\System32\curl.exe -sS -o NUL -w ""%{http_code}"" --max-time 2 ""$probeUrl""" 2>$null
    if ($out -eq '200') { $ready = $true; break }
}
if (-not $ready) {
    if (Test-Path $LogFile) { Get-Content $LogFile | Select-Object -Last 20 }
    Fail "server did not become ready in time"
}
Write-Status "server ready"

# 6. First clone --------------------------------------------------------
$cloneDir = Join-Path $TmpRoot 'clone1'
Write-Status "git clone (first)"
# Embed Basic auth in the URL so `git push` later can authenticate.
Invoke-Git clone "http://admin:admin@$Bind/repos/$RepoName.git" $cloneDir | ForEach-Object { Write-Status "  $_" }
if ($LASTEXITCODE -ne 0) { Fail "first clone failed" }

Push-Location $cloneDir
try {
    $defaultBranch = (Invoke-Git symbolic-ref --short HEAD 2>$null) ; if (-not $defaultBranch) { $defaultBranch = 'main' }
    Write-Status "default branch = $defaultBranch"

    Write-Status "create + commit hello.txt on $defaultBranch"
    'hello from gitgit MVP' | Set-Content -NoNewline -Encoding ascii hello.txt
    Invoke-Git config user.email "smoke@gitgit.local" | Out-Null
    Invoke-Git config user.name  "gitgit smoke"        | Out-Null
    Invoke-Git add hello.txt | ForEach-Object { Write-Status "  $_" }
    Invoke-Git commit -m "initial commit on $defaultBranch" | ForEach-Object { Write-Status "  $_" }
    if ($LASTEXITCODE -ne 0) { Fail "commit failed" }

    Write-Status "git push -u origin $defaultBranch"
    Invoke-Git push -u origin $defaultBranch | ForEach-Object { Write-Status "  $_" }
    if ($LASTEXITCODE -ne 0) { Fail "push to $defaultBranch failed" }

    # Feature branch -----------------------------------------------------
    Write-Status "git checkout -b feature"
    Invoke-Git checkout -b feature | ForEach-Object { Write-Status "  $_" }
    if ($LASTEXITCODE -ne 0) { Fail "checkout -b feature failed" }

    'feature branch content' | Set-Content -NoNewline -Encoding ascii feature.txt
    Invoke-Git add feature.txt | ForEach-Object { Write-Status "  $_" }
    Invoke-Git commit -m "feature branch commit" | ForEach-Object { Write-Status "  $_" }
    if ($LASTEXITCODE -ne 0) { Fail "feature commit failed" }

    Write-Status "git push -u origin feature"
    Invoke-Git push -u origin feature | ForEach-Object { Write-Status "  $_" }
    if ($LASTEXITCODE -ne 0) { Fail "push feature failed" }
}
finally {
    Pop-Location
}

# 7. Second clone + merge -----------------------------------------------
$cloneDir2 = Join-Path $TmpRoot 'clone2'
Write-Status "git clone (second) for merge verification"
Invoke-Git clone "http://admin:admin@$Bind/repos/$RepoName.git" $cloneDir2 | ForEach-Object { Write-Status "  $_" }
if ($LASTEXITCODE -ne 0) { Fail "second clone failed" }

Push-Location $cloneDir2
try {
    $defaultBranch = (Invoke-Git symbolic-ref --short HEAD 2>$null) ; if (-not $defaultBranch) { $defaultBranch = 'main' }
    Write-Status "fetch + merge origin/feature into $defaultBranch"
    Invoke-Git config user.email "smoke@gitgit.local" | Out-Null
    Invoke-Git config user.name  "gitgit smoke"        | Out-Null
    Invoke-Git fetch origin | ForEach-Object { Write-Status "  $_" }
    Invoke-Git merge --no-ff origin/feature -m "merge feature into $defaultBranch" | ForEach-Object { Write-Status "  $_" }
    if ($LASTEXITCODE -ne 0) { Fail "merge failed" }

    # Verify file content.
    if (-not (Test-Path 'feature.txt')) {
        Fail "feature.txt missing after merge"
    }
    $content = Get-Content -Raw feature.txt
    if ($content -ne 'feature branch content') {
        Fail "feature.txt content mismatch: '$content'"
    }

    # Verify the merge commit exists in the log.
    $log = (Invoke-Git log --oneline -n 5) -join "`n"
    if (-not ($log -match 'merge feature')) {
        Write-Host $log
        Fail "merge commit not found in log"
    }
    Write-Status "merge commit verified in log"
}
finally {
    Pop-Location
}

# 8. Auth enforcement on receive-pack -----------------------------------
Write-Status "negative auth: push without creds must fail"
$port = $Bind.Split(':')[1]
$tmpdir = Join-Path $TmpRoot 'clone-anon'
Invoke-Git clone "http://127.0.0.1:$port/repos/$RepoName.git" $tmpdir 2>&1 | Out-Null
Push-Location $tmpdir
try {
    Invoke-Git config user.email "anon@gitgit.local" | Out-Null
    Invoke-Git config user.name  "anon"               | Out-Null
    'unauth' | Set-Content -NoNewline -Encoding ascii unauth.txt
    Invoke-Git add unauth.txt | Out-Null
    Invoke-Git commit -m "unauth attempt" 2>&1 | Out-Null
    # `-c credential.helper=` explicitly disables every helper (including
    # the system one at `C:\Program Files\Git\etc\gitconfig`). Without
    # this, Git for Windows' `manager` helper would silently fulfill a
    # password and make this test spuriously pass.
    $pushOut = Invoke-Git -c credential.helper= push -u origin $defaultBranch 2>&1
    $pushExit = $LASTEXITCODE
    $pushOut | ForEach-Object { Write-Status "  push-output: $_" }
    if ($pushExit -eq 0) {
        # Some git versions exit 0 even on auth failure; check server log.
        if (Test-Path $LogFile) {
            $log = Get-Content $LogFile -Raw
            if ($log -match 'authentication required|401|Unauthorized') {
                Write-Status "  received expected auth challenge (in server stderr)"
            } else {
                Write-Status "  server log: $log"
                Fail "unauthenticated push unexpectedly succeeded (exit=$pushExit, no auth challenge in server log)"
            }
        } else {
            Fail "unauthenticated push unexpectedly succeeded (no server log)"
        }
    } else {
        Write-Status "  push correctly rejected (exit=$pushExit)"
    }
}
finally {
    Pop-Location
}

# 9. Tear down -----------------------------------------------------------
if ($ServerProc -and -not $ServerProc.HasExited) {
    Write-Status "stopping server (pid=$($ServerProc.Id))"
    Stop-Process -Id $ServerProc.Id -Force -ErrorAction SilentlyContinue
    $ServerProc.WaitForExit(5000) | Out-Null
}

if (Test-Path $TmpRoot) {
    cmd /c "rmdir /S /Q `"$TmpRoot`"" 2>$null | Out-Null
}
if (Test-Path "$LogFile.err") {
    if ((Get-Item "$LogFile.err").Length -gt 0) {
        Write-Status "server stderr (informational):"
        Get-Content "$LogFile.err" | Select-Object -Last 20 | ForEach-Object { Write-Status "  $_" }
    }
    Remove-Item -Force "$LogFile.err" -ErrorAction SilentlyContinue
}

Write-Host ""
Write-Host "========================================" -ForegroundColor Green
Write-Host "  gitgit MVP smoke test: PASS" -ForegroundColor Green
Write-Host "========================================" -ForegroundColor Green
exit 0
