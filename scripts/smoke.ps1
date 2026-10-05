#requires -Version 7
<#
.SYNOPSIS
    End-to-end smoke test for the `gitgit` MVP.

.DESCRIPTION
    1. Build the `gitgit` binary (debug profile, fast).
    2. Start it in the background listening on 127.0.0.1:8088 (override
       with -Bind). The port is deliberately different from the default
       8080 so the smoke test can run side-by-side with a long-lived
       dev server.
    3. `git clone http://<user>:<pass>@127.0.0.1:8088/repos/demo.git` to
       a temp directory.
    4. Create a file, commit, push to `main`.
    5. Create a `feature` branch, push.
    6. Clone a second time, fetch, merge origin/feature, verify the
       merge commit and the file content from the feature branch.
    7. Verify a push without credentials is rejected.
    8. Kill the server, print PASS / FAIL.

    Portable: runs on Windows pwsh 7 and on ubuntu-latest CI runners.
    External tools (git, cargo, curl) are resolved by name through
    scripts/lib/regression-common.ps1, so no path here is specific to
    one operating system.

    Two details worth keeping:
      * The readiness probe must not go through `Invoke-WebRequest`,
        which negotiates auth before sending a request and can mangle
        the `?` in the git query string. `Wait-HttpReady` in the common
        lib drives curl with an exact argv vector instead.
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
    [switch]$SkipBuild,
    # `[FACT]` The admin credential the server under test is started with.
    #
    # This existed as a literal `admin:admin` in the clone URLs until
    # 2026-10-05, when the compiled-in `config::ADMIN_USER` /
    # `ADMIN_PASS` pair was deleted. With no `GITGIT_ADMIN_PASS` in the
    # environment the server now generates a random 128-bit password, so
    # the literal in the URL no longer matched anything and `git push`
    # failed with 401 -- which is exactly what the `ST + IT regression`
    # CI job reported.
    #
    # The password is URL-safe (no `:`, `@`, `/` or `%`), so it is
    # embedded directly rather than percent-encoded. A value needing
    # encoding would have to be escaped here too, or the smoke test
    # would fail for a reason that has nothing to do with the server.
    [string]$AdminUser = 'admin',
    [string]$AdminPass = 'smoke-admin-pass'
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
. (Join-Path $PSScriptRoot 'lib/regression-common.ps1')

$ScratchTemp = Get-ScratchTempRoot

$env:HOME = Join-Path $ScratchTemp "gitgit-smoke-home-$([guid]::NewGuid().ToString('N'))"
New-Item -ItemType Directory -Path $env:HOME -Force | Out-Null
# Empty gitconfig so git has no helper at all.
$env:XDG_CONFIG_HOME = $env:HOME
# Drop any inherited credential helper selection.
$env:GIT_CONFIG_COUNT = '0'

$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$LogFile  = Join-Path $ScratchTemp "gitgit-smoke-$([guid]::NewGuid().ToString('N')).log"
$TmpRoot  = Join-Path $ScratchTemp "gitgit-smoke-$([guid]::NewGuid().ToString('N'))"
$ServerProc = $null

$GitCmd = Get-ToolPath -Name 'git'
if (-not $GitCmd) {
    Write-Error '[smoke] FAIL: git not found on PATH'
    exit 1
}

function Write-Status($msg) {
    Write-Host "[smoke] $msg"
}

function Fail($msg) {
    Write-Error "[smoke] FAIL: $msg"
    if ($ServerProc -and -not $ServerProc.HasExited) {
        try { Stop-Process -Id $ServerProc.Id -Force -ErrorAction SilentlyContinue } catch {}
    }
    Remove-ScratchRoot $TmpRoot
    exit 1
}

# Wrapper around `git` that captures stdout+stderr without PowerShell
# promoting stderr lines to terminating exceptions under Strict-Mode.
function Invoke-Git {
    param([Parameter(ValueFromRemainingArguments=$true)][string[]]$GitArgs)
    $local:ErrorActionPreference = 'Continue'
    $output = & $GitCmd @GitArgs 2>&1
    return $output
}

# 1. Build ---------------------------------------------------------------
if (-not $SkipBuild) {
    Write-Status "cargo build --locked --quiet (debug)"
    $cargo = Get-ToolPath -Name 'cargo'
    if (-not $cargo) {
        Fail 'cargo not found on PATH'
    }
    # --locked so a build in CI cannot quietly rewrite Cargo.lock.
    $build = Invoke-External -FilePath $cargo -TimeoutSec 600 `
        -WorkingDirectory $RepoRoot `
        -ArgumentList @('build', '--locked', '--quiet')
    if ($build.ExitCode -ne 0) {
        Write-Host ($build.StdOut + $build.StdErr)
        Fail "cargo build failed (exit=$($build.ExitCode))"
    }
}

# Locate the built binary. CARGO_TARGET_DIR may be set by the caller.
$exeSuffix = if (Test-IsWindows) { '.exe' } else { '' }
$targetDir = $env:CARGO_TARGET_DIR
if (-not $targetDir) { $targetDir = Join-Path $RepoRoot 'target' }
$bin = Join-Path $targetDir "debug/gitgit$exeSuffix"
if (-not (Test-Path $bin)) {
    Fail "binary not found at $bin"
}

# 2. Reset repos dir under a dedicated scratch ----------------------------
$reposDir = Join-Path $TmpRoot 'repos'
New-Item -ItemType Directory -Path $reposDir -Force | Out-Null

# 3. Start server --------------------------------------------------------
Write-Status "starting server on $Bind (log: $LogFile)"
# `[FACT]` The credential reaches the server through the environment,
# which `Start-Process` snapshots at spawn time. It is set immediately
# before the spawn and the previous value restored immediately after, so
# it does not leak into the `git` invocations later in this script -- a
# stray GITGIT_ADMIN_PASS in the caller's shell is exactly the kind of
# thing that makes a test pass for the wrong reason. The *previous* value
# is saved rather than just its presence: restoring "was set" by writing
# back the new value would silently re-point the caller's shell at a
# password only this test knew.
$prevAdminPass = $env:GITGIT_ADMIN_PASS
$prevAdminUser = $env:GITGIT_ADMIN_USER
$env:GITGIT_ADMIN_PASS = $AdminPass
$env:GITGIT_ADMIN_USER = $AdminUser
try {
    $ServerProc = Start-Process -FilePath $bin `
        -ArgumentList @('serve', '--bind', $Bind, '--repos-dir', $reposDir) `
        -PassThru `
        -NoNewWindow `
        -RedirectStandardOutput $LogFile `
        -RedirectStandardError "$LogFile.err"
}
finally {
    if ($null -eq $prevAdminPass) { Remove-Item Env:GITGIT_ADMIN_PASS -ErrorAction SilentlyContinue }
    else { $env:GITGIT_ADMIN_PASS = $prevAdminPass }
    if ($null -eq $prevAdminUser) { Remove-Item Env:GITGIT_ADMIN_USER -ErrorAction SilentlyContinue }
    else { $env:GITGIT_ADMIN_USER = $prevAdminUser }
}

# The credential as git sees it in a URL. Only `POST
# /git-receive-pack` is authenticated -- clone and `info/refs` are open
# by design -- so this is what the push in steps 4 and 5 authenticates
# with, and it is the reason the anonymous clone in step 8 still works.
$Credential = '{0}:{1}' -f $AdminUser, $AdminPass

# 4. init-repo via the CLI BEFORE the readiness probe. The probe hits
#    /repos/<name>.git/info/refs, which 404s if the bare repo doesn't
#    exist yet on disk.
Write-Status "gitgit init-repo $RepoName"
$initOut = & $bin init-repo $RepoName --repos-dir $reposDir 2>&1
if ($LASTEXITCODE -ne 0) { Fail "init-repo failed: $($initOut -join "`n")" }
if (-not (Test-Path (Join-Path $reposDir "$RepoName.git/HEAD"))) {
    Fail "bare repo HEAD not found at $reposDir/$RepoName.git/HEAD"
}

# 5. Wait until /info/refs responds 200 (with a query string) ------------
$probeUrl = "http://$Bind/repos/$RepoName.git/info/refs?service=git-upload-pack"
Write-Status "waiting for $probeUrl"
$ready = Wait-HttpReady -Url $probeUrl -MaxAttempts 50 -DelayMs 200
if (-not $ready) {
    if (Test-Path $LogFile) { Get-Content $LogFile | Select-Object -Last 20 }
    Fail "server did not become ready in time"
}
Write-Status "server ready"

# 6. First clone --------------------------------------------------------
$cloneDir = Join-Path $TmpRoot 'clone1'
Write-Status "git clone (first)"
# Embed Basic auth in the URL so `git push` later can authenticate.
Invoke-Git clone "http://$Credential@$Bind/repos/$RepoName.git" $cloneDir | ForEach-Object { Write-Status "  $_" }
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
Invoke-Git clone "http://$Credential@$Bind/repos/$RepoName.git" $cloneDir2 | ForEach-Object { Write-Status "  $_" }
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

Remove-ScratchRoot $TmpRoot
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
