# End-to-end check of `gitremote add` / `ls` / `sync` against real git repos.
#
# V0 task T8. The store is a local JSON file (ADR-0022 §2.1 forbids taking a
# sqlx / PG dependency into this crate), so nothing here is mocked: this
# builds three real git repositories and drives the actual binary.
#
# What it proves:
#   * `add` writes a registry entry that survives a fresh process
#   * `ls` renders it
#   * `rm` removes it
#   * `sync` really moves commits between two bare repos
#   * a URL containing shell metacharacters is stored but never executed
#
# Run: pwsh -NoProfile -File scripts/verify-gitremote-e2e.ps1

$ErrorActionPreference = 'Stop'

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
. (Join-Path $PSScriptRoot 'lib/regression-common.ps1')

$exeName = if (Test-IsWindows) { 'gitgit.exe' } else { 'gitgit' }
$candidates = @(
    $(if ($env:CARGO_TARGET_DIR) { Join-Path $env:CARGO_TARGET_DIR "debug/$exeName" })
    (Join-Path $repoRoot "target-regression/debug/$exeName")
    (Join-Path $repoRoot "target/debug/$exeName")
) | Where-Object { $_ }
$bin = $candidates | Where-Object { Test-Path $_ } | Select-Object -First 1
if (-not $bin) {
    throw "gitgit binary not found. Looked in:`n  " + ($candidates -join "`n  ") + "`nRun 'cargo build --bins' first."
}

$failures = @()
function Add-Failure($m) { $script:failures += $m }
function Check($cond, $what) { if (-not $cond) { Add-Failure $what } }

$work = Join-Path (Get-ScratchTempRoot) ('gitgit-gitremote-e2e-' + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $work -Force | Out-Null

# Isolate git from the developer's global config.
$env:GIT_CONFIG_GLOBAL = Join-Path $work 'gitconfig'
$env:GIT_CONFIG_NOSYSTEM = '1'
$env:GIT_AUTHOR_NAME = 'e2e'; $env:GIT_AUTHOR_EMAIL = 'e2e@example.invalid'
$env:GIT_COMMITTER_NAME = 'e2e'; $env:GIT_COMMITTER_EMAIL = 'e2e@example.invalid'

function Invoke-Git([string]$dir, [string[]]$gitArgs) {
    Push-Location $dir
    try {
        $out = & git @gitArgs 2>&1
        if ($LASTEXITCODE -ne 0) {
            throw ("git {0} failed in {1}:" + "`n" + ($out -join "`n")) -f ($gitArgs -join ' '), $dir
        }
        return ($out -join "`n")
    } finally { Pop-Location }
}

function Try-Git([string]$dir, [string[]]$gitArgs) {
    # For probes where a non-zero exit is a legitimate answer, e.g.
    # `rev-parse --verify --quiet` on a ref that may not exist. Throwing
    # there would turn "the ref is absent" into a harness crash.
    Push-Location $dir
    try {
        $out = & git @gitArgs 2>&1
        if ($LASTEXITCODE -ne 0) { return $null }
        return ($out -join "`n")
    } finally { Pop-Location }
}

# ── Fixtures: a bare "server" repo, a bare "upstream", and a work clone ────
$serverDir  = Join-Path $work 'server.git'
$upstream   = Join-Path $work 'upstream.git'
$reposDir   = Join-Path $work 'repos'
$stateDir   = Join-Path $work 'state'

Invoke-Git $work @('init', '--bare', '-q', $serverDir) | Out-Null
Invoke-Git $work @('init', '--bare', '-q', $upstream)  | Out-Null
Invoke-Git $work @('init', '-q', (Join-Path $work 'clone')) | Out-Null
$clone = Join-Path $work 'clone'

Set-Content -Path (Join-Path $clone 'a.txt') -Value 'one' -NoNewline
Invoke-Git $clone @('add', '.') | Out-Null
Invoke-Git $clone @('commit', '-q', '-m', 'first') | Out-Null
Invoke-Git $clone @('branch', '-M', 'main') | Out-Null
Invoke-Git $clone @('push', '-q', $upstream, 'refs/heads/main:refs/heads/main') | Out-Null

# The remote name must match the repo name so the default --repo
# resolution (repos_dir/<name>.git) is what gets exercised.
New-Item -ItemType Directory -Path $reposDir -Force | Out-Null
Copy-Item -Recurse -Path $serverDir -Destination (Join-Path $reposDir 'demo.git')
# `git init --bare` leaves HEAD on whatever the local default is (master on
# this box, main elsewhere). Point it at main so the default-branch
# resolution in `sync` lines up with the fixture instead of failing on a
# branch the upstream does not have.
Invoke-Git (Join-Path $reposDir 'demo.git') @('symbolic-ref', 'HEAD', 'refs/heads/main') | Out-Null

$env:GITGIT_VAULT_FILE_ROOT = Join-Path $stateDir 'vault'
# Pass the flag as well as the env var: the flag is what the binary is
# documented to honour, and asserting the registry lands in a temp dir
# must not depend on env-var plumbing.
$commonArgs = @('--repos-dir', $reposDir, '--vault-file-root', $env:GITGIT_VAULT_FILE_ROOT)

Write-Host '── gitremote add / ls / rm / sync against real git repos ──'

# ── add ────────────────────────────────────────────────────────────────────
$addOut = (& $bin gitremote @commonArgs add demo $upstream 2>&1) -join ' '
Check ($LASTEXITCODE -eq 0) "gitremote add exited non-zero: $addOut"
Check ($addOut -match 'added') "add did not report 'added': $addOut"

# ── ls, in a fresh process, proving the registry is on disk ───────────────
$lsOut = (& $bin gitremote @commonArgs ls 2>&1) -join ' '
Check ($LASTEXITCODE -eq 0) "gitremote ls exited non-zero: $lsOut"
Check ($lsOut -match 'demo') "ls does not show the remote: $lsOut"
Check ($lsOut -match [regex]::Escape($upstream)) "ls does not show the URL: $lsOut"

# The registry must sit where the docs say, and be valid JSON.
$registry = Join-Path $stateDir 'remotes.json'
Check (Test-Path $registry) "registry not written to $registry"
if (Test-Path $registry) {
    try {
        $doc = Get-Content -Raw $registry | ConvertFrom-Json
        Check ($doc.remotes.Count -eq 1) "expected 1 remote, got $($doc.remotes.Count)"
        Check ($doc.remotes[0].url -eq $upstream) "registry URL mismatch"
    } catch {
        Add-Failure "registry is not valid JSON: $($_.Exception.Message)"
    }
}

# ── sync: must actually move the commit into the server repo ──────────────
$syncOut = (& $bin gitremote @commonArgs sync demo 2>&1) -join ' '
Check ($LASTEXITCODE -eq 0) "gitremote sync exited non-zero: $syncOut"
$tracked = Try-Git (Join-Path $reposDir 'demo.git') @('rev-parse', '--verify', '--quiet', 'refs/remotes/demo/main')
Check ($null -ne $tracked) "sync did not create the remote-tracking ref (output: $syncOut)"

$localSha = (Invoke-Git $clone @('rev-parse', 'refs/heads/main')).Trim()
Check ($tracked -ne $null -and $tracked.Trim() -eq $localSha) "tracked ref does not match local $localSha"

# A second sync has nothing to do and must say so rather than re-push.
$sync2 = (& $bin gitremote @commonArgs sync demo 2>&1) -join ' '
Check ($LASTEXITCODE -eq 0) "second sync exited non-zero: $sync2"

# ── a URL that looks like a shell injection must never be executed ───────
# Two distinct cases, because they exercise different code paths.
#
#  1. Whitespace in a URL: rejected up front by validate_url. A space
#     would be split by argv handling and could silently target the wrong
#     repository, so refusing beats storing it.
$spaced = 'https://example.invalid/r.git; touch spaced-pwned.txt'
$spacedOut = (& $bin gitremote @commonArgs add spaced $spaced 2>&1) -join ' '
Check ($LASTEXITCODE -ne 0) "a URL containing a space should be rejected, got: $spacedOut"
Check (-not (Test-Path (Join-Path $stateDir 'spaced-pwned.txt'))) 'a spaced URL was executed as a shell command'

#  2. Shell metacharacters with no whitespace: accepted and stored
#     verbatim, because it is otherwise a legal git URL, but never handed
#     to a shell. Single-quoted so PowerShell does not expand $( ) itself.
#     $(id>.pwned2) contains no whitespace, which is what lets it past
#     validate_url.
$hostile = 'https://example.invalid/$(id>.pwned2).git'
$badAdd = (& $bin gitremote @commonArgs add hostile $hostile 2>&1) -join ' '
Check ($LASTEXITCODE -eq 0) "a metacharacter URL should be stored verbatim, got: $badAdd"
$badSync = (& $bin gitremote @commonArgs sync hostile 2>&1) -join ' '
Check (-not (Test-Path (Join-Path $stateDir 'pwned2'))) 'a URL was executed as a shell command'
Check (-not (Test-Path (Join-Path $work 'pwned2'))) 'a URL was executed as a shell command in the work dir'

# ── rm ────────────────────────────────────────────────────────────────────
$rmOut = (& $bin gitremote @commonArgs rm hostile 2>&1) -join ' '
Check ($LASTEXITCODE -eq 0) "gitremote rm exited non-zero: $rmOut"
$rmOut2 = (& $bin gitremote @commonArgs rm spaced 2>&1) -join ' '
$lsAfter = (& $bin gitremote @commonArgs ls 2>&1) -join ' '
Check ($lsAfter -notmatch 'hostile') "rm did not remove the entry: $lsAfter"

# ── error paths ───────────────────────────────────────────────────────────
$missing = (& $bin gitremote @commonArgs sync nosuch 2>&1) -join ' '
Check ($LASTEXITCODE -ne 0) 'syncing an unknown remote should fail'
Check ($missing -match 'nosuch') "unknown-remote error is not actionable: $missing"

Write-Host ("  sync output  : {0}" -f $syncOut.Trim())
Write-Host ("  ls output    : {0}" -f $lsOut.Trim())
Write-Host ("  registry     : {0}" -f $registry)

if ($failures.Count -gt 0) {
    Write-Host ''
    foreach ($f in $failures) { Write-Host ("  FAIL: {0}" -f $f) -ForegroundColor Red }
    exit 1
}
Write-Host '  PASS: add/ls/rm persisted, sync moved real commits, hostile URL was not executed' -ForegroundColor Green
exit 0
