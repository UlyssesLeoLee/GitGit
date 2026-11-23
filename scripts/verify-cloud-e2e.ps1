#requires -Version 7
<#
.SYNOPSIS
    Drive the `gitgit cloud` binary end to end against this repository's own
    object store.

.DESCRIPTION
    The cloud-disk feature has library-level end-to-end tests
    (`src/cloud/backup.rs` drives a real S3-compatible endpoint), but the
    `gitgit cloud` **binary** had never been exercised end to end. The
    `Handle::current().block_on` panic in every `gitgit key *` subcommand
    went unnoticed for exactly that reason, and
    `scripts/verify-gitai-e2e.ps1` and `scripts/verify-gitremote-e2e.ps1`
    exist to close that same gap for their own subcommands.

    This script closes it for the cloud disk. It starts this repository's
    own `gitgit-objectstore`, builds a real bare repository with real git,
    and drives the product binary through the whole operator path:

      1. `cloud add`       - connect a disk. The credentials travel in the
                             environment and never on a command line.
      2. `cloud list`      - the disk shows up with its endpoint, bucket and
                             key prefix.
      3. `cloud backup`    - the bare repository is uploaded, and the
                             snapshot key is parsed out of the output.
      4. (damage)         - the repository is genuinely broken: its HEAD is
                             pointed at a branch that does not exist, its
                             branch ref is deleted, and a marker file is
                             left behind.
      5. `cloud snapshots` - the snapshot the backup just took is listed.
      6. `cloud restore`   - the repository comes back exactly as it was.
      7. `cloud rm`        - the disk is disconnected, the registry no
                             longer knows it, and the snapshot is still on
                             the remote.

    What this script deliberately does not claim:

      * It does not test the library. `src/cloud/backup.rs` does that; this
        script says so rather than duplicating the coverage.
      * It does not claim the rollback works against a hosted S3 service.
        The endpoint is this repository's own object store - the same one
        `scripts/verify-s3-vault.ps1` uses for the credential vault.

    Fail-closed note: every step asserts the exit code AND the output, an
    empty output is a failure (`Assert-Match` refuses an empty `Actual`),
    and the run asserts at the end that all ten steps ran and that the run
    made the number of assertions it expected. A step that quietly did
    nothing therefore cannot turn this into a green no-op.

.PARAMETER TargetDir
    The Cargo target directory to build into. Defaults to
    `<repo>/target-regression`, the directory this repository's own
    regression tiers build into - so the CI integration job, whose IT tier
    has already compiled the crate, does not pay for a second cold
    compile.

.EXAMPLE
    pwsh -NoProfile -File scripts/verify-cloud-e2e.ps1
#>

[CmdletBinding()]
param(
    [string]$TargetDir
)

$ErrorActionPreference = 'Stop'

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
. (Join-Path $PSScriptRoot 'lib/regression-common.ps1')

$run = New-RegressionRun -Tier 'cloud-e2e'

# ── the fixtures this run uses ──────────────────────────────────────────────
$diskName  = 'e2e-disk'
$bucket    = 'gitgit-cloud-e2e'
$prefix    = 'e2e/'
$region    = 'us-east-1'
$repoName  = 'demo'
$branch    = 'main'
$markerName = 'DAMAGED-BY-VERIFY-CLOUD-E2E'

# [FACT] The object store's access key is an identifier, not a secret, so
# it may be an argument (`src/bin/gitgit-objectstore.rs`). The secret may
# not: that binary has no `--secret-key` flag and refuses to start without
# `GITGIT_OBJECTSTORE_SECRET_KEY` in the environment.
$accessKey = 'gitgit-cloud-e2e-access'
# [FACT] A fresh secret for every run: this is a throwaway store, and a
# secret that outlives the run is a secret that can outlive its owner.
$secretKey = [Convert]::ToBase64String([System.Security.Cryptography.RandomNumberGenerator]::GetBytes(32))

function Get-CloudEnvSuffix {
    <#
      The disk name as it appears in an environment variable name.

      [FACT] The product's rule (`src/cloud/mod.rs`, `env_suffix`): every
      ASCII alphanumeric character is kept and upper-cased, every other
      character becomes `_`. This is an independent implementation of that
      rule on purpose: the script asserts that the product prints the names
      it computes, and if both sides used one implementation the assertion
      would be true by construction rather than by observation.
    #>
    param([Parameter(Mandatory)][string]$Name)
    $sb = New-Object System.Text.StringBuilder
    foreach ($ch in $Name.ToCharArray()) {
        if ($ch -match '[A-Za-z0-9]') { [void]$sb.Append([char]::ToUpperInvariant($ch)) }
        else { [void]$sb.Append('_') }
    }
    return $sb.ToString()
}

$envSuffix    = Get-CloudEnvSuffix -Name $diskName
$accessKeyEnv = "GITGIT_CLOUD_${envSuffix}_ACCESS_KEY"
$secretKeyEnv = "GITGIT_CLOUD_${envSuffix}_SECRET_KEY"

# ── helpers ─────────────────────────────────────────────────────────────────
$script:stepCount = 0
function Step {
    param([string]$Message)
    $script:stepCount++
    Write-Step ("({0}/10) {1}" -f $script:stepCount, $Message)
}

function Assert-Literal {
    <#
      A "the output contains this exact text" assertion.

      [FACT] The comparison is `String.Contains`, so it is case-SENSITIVE
      and literal: a key, an endpoint or a variable name has to be spelled
      the way the product spelled it. (PowerShell's `-match` is
      case-insensitive, which would let `PREFIX=e2e/` satisfy an assertion
      that claims to have seen `prefix=e2e/`.)

      [FACT] An empty expectation is refused rather than honoured: an
      assertion whose expected text is empty matches every output, so it
      would pass without proving anything.
    #>
    param(
        [Parameter(Mandatory)][string]$Name,
        [Parameter(Mandatory)][AllowEmptyString()][string]$Expected,
        [Parameter(Mandatory)][AllowEmptyString()][string]$Actual,
        [string]$Detail = ''
    )
    if ([string]::IsNullOrEmpty($Expected)) {
        Add-RegressionCase -Run $run -Name $Name -Outcome fail `
            -Detail 'the expected text was empty; such an assertion matches every output'
        Write-Fail "$Name :: the expected text was empty"
        return
    }
    $shown = if ($Actual.Length -gt 400) { $Actual.Substring(0, 400) + '...' } else { $Actual }
    $found = $Actual.Contains($Expected)
    Assert-True -Run $run -Name $Name -Condition $found `
        -Detail ("$Detail expected to contain <$Expected>, got <$shown>")
}

$cargo = Get-ToolPath -Name 'cargo'
if (-not $cargo) {
    throw 'cargo not found on PATH. This check builds the product binary from this repository.'
}
$git = Get-ToolPath -Name 'git'
if (-not $git) {
    throw 'git not found on PATH. This check builds a real bare repository with real git.'
}

# [FACT] The build goes into <repo>/target-regression - the directory
# scripts/regression-ut.ps1, scripts/regression-it.ps1 and
# scripts/regression-st.ps1 build into - and NOT into a shared
# CARGO_TARGET_DIR. The process-wide CARGO_TARGET_DIR is overridden here
# for the same reason those tiers override it: concurrent lanes would
# otherwise fight over one cargo file lock. The CI integration job runs
# those tiers before this script, so the compile is already warm.
if (-not $TargetDir) { $TargetDir = Join-Path $repoRoot 'target-regression' }
$env:CARGO_TARGET_DIR = $TargetDir

# ── the scratch tree everything lives in ────────────────────────────────────
$work = New-ScratchRoot -Prefix 'gitgit-cloud-e2e'
$reposDir    = Join-Path $work 'repos'
$vaultRoot   = Join-Path $work 'state/vault'
$storeRoot   = Join-Path $work 'objectstore'
$bareDir     = Join-Path $reposDir "$repoName.git"
$workTree    = Join-Path $work 'work'
$registryPath = Join-Path (Split-Path -Parent $vaultRoot) 'clouds.json'
New-Item -ItemType Directory -Path $reposDir -Force | Out-Null
New-Item -ItemType Directory -Path $vaultRoot -Force | Out-Null
New-Item -ItemType Directory -Path $storeRoot -Force | Out-Null

# [FACT] The cloud disk registry sits next to the vault root, not inside
# it: `cloud_registry_path` (src/main.rs) is `parent(--vault-file-root)` +
# `clouds.json`. The script computes that path itself rather than reading
# it out of the product's output, so the "the registry no longer knows the
# disk" assertion lands on the file the product actually wrote.
$expectedRegistryPath = Join-Path (Split-Path -Parent $vaultRoot) 'clouds.json'

# Isolate git from the developer's global configuration, the way
# scripts/verify-gitremote-e2e.ps1 does.
$env:GIT_CONFIG_GLOBAL  = Join-Path $work 'gitconfig'
$env:GIT_CONFIG_NOSYSTEM = '1'
$env:GIT_AUTHOR_NAME    = 'e2e'
$env:GIT_AUTHOR_EMAIL   = 'e2e@example.invalid'
$env:GIT_COMMITTER_NAME = 'e2e'
$env:GIT_COMMITTER_EMAIL = 'e2e@example.invalid'

function Invoke-Git {
    param([string]$Dir, [string[]]$GitArgs)
    $r = Invoke-External -FilePath $git -WorkingDirectory $Dir -ArgumentList $GitArgs
    if ($r.ExitCode -ne 0) {
        throw ("git {0} failed in {1} (exit {2}): {3}" -f ($GitArgs -join ' '), $Dir, $r.ExitCode, ($r.StdErr.Trim()))
    }
    return $r.StdOut.Trim()
}

function Try-Git {
    <#  For probes where a non-zero exit is a legitimate answer - e.g.
        `rev-parse --verify` on a ref that was just deleted.  #>
    param([string]$Dir, [string[]]$GitArgs)
    $r = Invoke-External -FilePath $git -WorkingDirectory $Dir -ArgumentList $GitArgs
    if ($r.ExitCode -ne 0) { return $null }
    return $r.StdOut.Trim()
}

$gitgitArgs = @('--repos-dir', $reposDir, '--vault-file-root', $vaultRoot)

function Invoke-GitGit {
    <#  Run the product binary and make sure the secret never came back
        in anything it printed.

        [FACT] The parameter is named `$SubArgs` and NOT `$Args`: `$args`
        is PowerShell's reserved automatic variable, and a parameter
        declared with that name does not bind the positional argument —
        the array lands in the automatic variable instead and the
        declared one stays empty. The command then runs with no
        subcommand at all, and clap answers "requires a subcommand but
        one was not provided" with exit code 2. It also cannot be called
        `$gitgitArgs`: PowerShell's variable names are case-insensitive,
        so that would collide with the script-level global of the same
        spelling.
    #>
    param([string[]]$SubArgs, [int]$TimeoutSec = 180)
    $r = Invoke-External -FilePath $script:gitgitBin -WorkingDirectory $repoRoot `
            -TimeoutSec $TimeoutSec -ArgumentList ($gitgitArgs + $SubArgs)
    if ($r.StdOut -and $r.StdOut.Contains($secretKey)) {
        Add-RegressionCase -Run $run -Name 'the secret key never appears in the binary output' `
            -Outcome fail -Detail 'the secret key was found in stdout'
        Write-Fail 'the secret key never appears in the binary output :: the secret key was found in stdout'
    }
    if ($r.StdErr -and $r.StdErr.Contains($secretKey)) {
        Add-RegressionCase -Run $run -Name 'the secret key never appears in the binary error output' `
            -Outcome fail -Detail 'the secret key was found in stderr'
        Write-Fail 'the secret key never appears in the binary error output :: the secret key was found in stderr'
    }
    return $r
}

$exitCode = 0
$server = $null
$serverLog = Join-Path $storeRoot 'objectstore.log'

try {
    # ══ step 1 — build the two binaries ═════════════════════════════════════
    Step 'building gitgit and gitgit-objectstore'
    $build = Invoke-External -FilePath $cargo -WorkingDirectory $repoRoot -TimeoutSec 1800 `
        -ArgumentList @('build', '--locked', '--bin', 'gitgit', '--bin', 'gitgit-objectstore')
    Assert-Equal -Run $run -Name 'cargo build --locked --bin gitgit --bin gitgit-objectstore exits 0' `
        -Expected 0 -Actual $build.ExitCode `
        -Detail ("cargo build exited $($build.ExitCode): " + (($build.StdErr + $build.StdOut).Trim()))
    if ($build.ExitCode -ne 0) {
        Write-Host ($build.StdErr + $build.StdOut)
    }

    $exeSuffix = if (Test-IsWindows) { '.exe' } else { '' }
    $script:gitgitBin        = Join-Path $TargetDir "debug/gitgit$exeSuffix"
    $script:objectStoreBin  = Join-Path $TargetDir "debug/gitgit-objectstore$exeSuffix"
    Assert-True -Run $run -Name 'the gitgit binary was built' `
        -Condition (Test-Path -LiteralPath $script:gitgitBin) `
        -Detail "expected a binary at $($script:gitgitBin)"
    Assert-True -Run $run -Name 'the gitgit-objectstore binary was built' `
        -Condition (Test-Path -LiteralPath $script:objectStoreBin) `
        -Detail "expected a binary at $($script:objectStoreBin)"

    # ══ step 2 — start this repository's own object store ═══════════════════
    Step "starting the object store"

    function Get-FreeTcpPort {
        # Port 0 lets the OS pick, so a parallel run cannot collide with
        # this one. The listener is closed before the server binds it - a
        # small race, and the reason the readiness probe below is a retry
        # loop and not a single connect.
        $listener = [System.Net.Sockets.TcpListener]::new([System.Net.IPAddress]::Loopback, 0)
        $listener.Start()
        try { return $listener.LocalEndpoint.Port } finally { $listener.Stop() }
    }

    $port     = Get-FreeTcpPort
    $bind     = "127.0.0.1:$port"
    $endpoint = "http://$bind"

    $psi = [System.Diagnostics.ProcessStartInfo]::new()
    $psi.FileName               = $script:objectStoreBin
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
    Assert-True -Run $run -Name 'the object store process starts' `
        -Condition ($null -ne $server) -Detail 'the process could not be started'

    $storeReady = $false
    for ($i = 0; $i -lt 80; $i++) {
        Start-Sleep -Milliseconds 250
        if ($server -and $server.HasExited) { break }
        try {
            $client = [System.Net.Sockets.TcpClient]::new()
            $client.Connect([System.Net.IPAddress]::Loopback, $port)
            $client.Dispose()
            $storeReady = $true
            break
        } catch {
            # not up yet
        }
    }
    Assert-True -Run $run -Name 'the object store is listening on its port' `
        -Condition $storeReady -Detail "the object store never started listening on $bind"

    # ══ step 3 — connect the disk ══════════════════════════════════════════
    Step 'connecting the cloud disk (cloud add)'

    # The credentials travel in the environment. The product reads them
    # from `GITGIT_CLOUD_<NAME>_ACCESS_KEY` / `..._SECRET_KEY`
    # (src/cloud/mod.rs); nothing is ever passed on a command line.
    Set-Item -Path "Env:$accessKeyEnv" -Value $accessKey
    Set-Item -Path "Env:$secretKeyEnv" -Value $secretKey

    $add = Invoke-GitGit @('cloud', 'add', $diskName,
        '--endpoint', $endpoint,
        '--bucket', $bucket,
        '--region', $region,
        '--prefix', $prefix)
    Assert-Equal -Run $run -Name 'gitgit cloud add exits 0' -Expected 0 -Actual $add.ExitCode `
        -Detail $add.StdErr.Trim()
    Assert-Literal -Name 'gitgit cloud add reports the disk it connected' `
        -Expected "connected $diskName" -Actual $add.StdOut
    Assert-Literal -Name 'gitgit cloud add reports the disk own access key variable' `
        -Expected $accessKeyEnv -Actual $add.StdOut
    Assert-Literal -Name 'gitgit cloud add reports the disk own secret key variable' `
        -Expected $secretKeyEnv -Actual $add.StdOut
    Assert-Literal -Name 'gitgit cloud add reports where the registry lives' `
        -Expected $expectedRegistryPath -Actual $add.StdOut
    Assert-True -Run $run -Name 'the cloud disk registry file was written' `
        -Condition (Test-Path -LiteralPath $registryPath) `
        -Detail "expected a registry at $registryPath"

    # ══ step 4 — list the disks ════════════════════════════════════════════
    Step 'listing the connected disks (cloud list)'
    $list = Invoke-GitGit @('cloud', 'list')
    Assert-Equal -Run $run -Name 'gitgit cloud list exits 0' -Expected 0 -Actual $list.ExitCode `
        -Detail $list.StdErr.Trim()
    Assert-Literal -Name 'gitgit cloud list names the connected disk' `
        -Expected $diskName -Actual $list.StdOut
    Assert-Literal -Name 'gitgit cloud list shows the disk endpoint' `
        -Expected $endpoint -Actual $list.StdOut
    Assert-Literal -Name 'gitgit cloud list shows the disk bucket' `
        -Expected "bucket=$bucket" -Actual $list.StdOut
    Assert-Literal -Name 'gitgit cloud list shows the disk key prefix' `
        -Expected "prefix=$prefix" -Actual $list.StdOut

    # ══ step 5 — a bare repository with real content ═══════════════════════
    Step 'building a real bare repository with real git'

    Invoke-Git $work @('init', '--bare', '-q', $bareDir) | Out-Null
    # `git init --bare` leaves HEAD on whatever the local default is
    # (master on this box, main elsewhere); point it at main so the
    # fixture matches the branch the script damages.
    Invoke-Git $bareDir @('symbolic-ref', 'HEAD', "refs/heads/$branch") | Out-Null

    Invoke-Git $work @('init', '-q', $workTree) | Out-Null
    Set-Content -LiteralPath (Join-Path $workTree 'hello.txt') -Value 'cloud e2e payload' -NoNewline
    Invoke-Git $workTree @('add', '.') | Out-Null
    Invoke-Git $workTree @('commit', '-q', '-m', 'cloud e2e fixture') | Out-Null
    Invoke-Git $workTree @('branch', '-M', $branch) | Out-Null
    Invoke-Git $workTree @('push', '-q', $bareDir, "$($branch):$($branch)") | Out-Null

    $commitSha = Invoke-Git $bareDir @('rev-parse', '--verify', "refs/heads/$branch")
    $blobSha   = Invoke-Git $bareDir @('rev-parse', '--verify', ('refs/heads/{0}:hello.txt' -f $branch))
    $blobBody  = Invoke-Git $bareDir @('cat-file', '-p', $blobSha)

    Assert-True -Run $run -Name 'the fixture bare repository holds a real commit' `
        -Condition ($commitSha -match '^[0-9a-f]{40}$') -Detail "git reported '$commitSha'"
    Assert-Equal -Run $run -Name 'git reports the fixture commit object as a commit' `
        -Expected 'commit' -Actual (Try-Git $bareDir @('cat-file', '-t', $commitSha))
    Assert-Equal -Run $run -Name 'git reports the fixture blob object as a blob' `
        -Expected 'blob' -Actual (Try-Git $bareDir @('cat-file', '-t', $blobSha))
    # The blob's own loose-object path is derived from its SHA-1, so this
    # points at the file on disk and not at a git report about it.
    $blobLoosePath = Join-Path (Join-Path (Join-Path $bareDir 'objects') $blobSha.Substring(0, 2)) $blobSha.Substring(2)
    Assert-True -Run $run -Name 'the fixture blob really sits under objects/ on disk' `
        -Condition (Test-Path -LiteralPath $blobLoosePath) -Detail "expected $blobLoosePath"
    $refIsLoose = Test-Path -LiteralPath (Join-Path (Join-Path $bareDir 'refs/heads') $branch)
    $refIsPacked = (Test-Path -LiteralPath (Join-Path $bareDir 'packed-refs')) -and
                   ((Get-Content -Raw -LiteralPath (Join-Path $bareDir 'packed-refs')) -match [regex]::Escape($commitSha))
    Assert-True -Run $run -Name 'the fixture branch ref exists under refs/heads/' `
        -Condition ($refIsLoose -or $refIsPacked) -Detail 'neither a loose ref nor a packed-refs entry'

    $headBefore = (Get-Content -Raw -LiteralPath (Join-Path $bareDir 'HEAD')).Trim()
    Assert-Equal -Run $run -Name 'the fixture HEAD points at the branch that holds the commit' `
        -Expected "ref: refs/heads/$branch" -Actual $headBefore

    # ══ step 6 — back the repository up ════════════════════════════════════
    Step 'backing the repository up (cloud backup)'
    $backup = Invoke-GitGit @('cloud', 'backup', '--disk', $diskName, '--repo', $repoName)
    Assert-Equal -Run $run -Name 'gitgit cloud backup exits 0' -Expected 0 -Actual $backup.ExitCode `
        -Detail $backup.StdErr.Trim()

    $backupKeyMatch = [regex]::Match($backup.StdOut, '(?m)^' + [regex]::Escape($repoName) + ' -> (?<key>snapshots/\S+\.tar)\s*$')
    Assert-True -Run $run -Name 'gitgit cloud backup reports the snapshot key it stored' `
        -Condition $backupKeyMatch.Success `
        -Detail "expected a line '<repo> -> snapshots/...tar', got: $($backup.StdOut.Trim())"
    $snapshotKey = if ($backupKeyMatch.Success) { $backupKeyMatch.Groups['key'].Value } else { '' }
    Assert-Literal -Name 'the snapshot key names the repository it holds' `
        -Expected "snapshots/$repoName/" -Actual $snapshotKey

    $backupMetaMatch = [regex]::Match($backup.StdOut, '(?m)^\s*(?<entries>\d+) files, sha256 (?<sha>[0-9a-f]{64})\s*$')
    Assert-True -Run $run -Name 'gitgit cloud backup reports the file count and the digest of the snapshot' `
        -Condition $backupMetaMatch.Success `
        -Detail "expected a '<n> files, sha256 <hex>' line, got: $($backup.StdOut.Trim())"
    if ($backupMetaMatch.Success) {
        Assert-True -Run $run -Name 'the snapshot really holds the repository files' `
            -Condition ([int]$backupMetaMatch.Groups['entries'].Value -ge 1) `
            -Detail "the snapshot claims to hold $($backupMetaMatch.Groups['entries'].Value) files"
    }

    # ══ step 7 — damage the repository ═════════════════════════════════════
    Step 'damaging the repository'
    $markerPath = Join-Path $bareDir $markerName

    # (a) point HEAD at a branch that does not exist
    Invoke-Git $bareDir @('symbolic-ref', 'HEAD', 'refs/heads/damaged') | Out-Null
    # (b) delete the branch ref the snapshot was taken with
    Invoke-Git $bareDir @('update-ref', '-d', "refs/heads/$branch") | Out-Null
    # (c) leave a marker file behind
    Set-Content -LiteralPath $markerPath -Value 'damaged by verify-cloud-e2e' -NoNewline

    $headDamaged = (Get-Content -Raw -LiteralPath (Join-Path $bareDir 'HEAD')).Trim()
    $refDamaged  = Try-Git $bareDir @('rev-parse', '--verify', "refs/heads/$branch")
    $headResolve = Try-Git $bareDir @('rev-parse', '--verify', 'HEAD')

    Assert-True -Run $run -Name 'the damage moved HEAD' `
        -Condition ($headDamaged -ne $headBefore) `
        -Detail "HEAD is '$headDamaged', it was '$headBefore'"
    Assert-True -Run $run -Name 'the damage deleted the branch ref' `
        -Condition ([string]::IsNullOrEmpty($refDamaged)) `
        -Detail "refs/heads/$branch still resolves to '$refDamaged'"
    Assert-True -Run $run -Name 'the damage left its marker file behind' `
        -Condition (Test-Path -LiteralPath $markerPath) -Detail "expected $markerPath"
    Assert-True -Run $run -Name 'the repository no longer resolves HEAD after the damage' `
        -Condition ([string]::IsNullOrEmpty($headResolve)) `
        -Detail "HEAD still resolves to '$headResolve'"

    # ══ step 8 — list the snapshots ════════════════════════════════════════
    Step 'listing the snapshots (cloud snapshots)'
    $snapshots = Invoke-GitGit @('cloud', 'snapshots', '--disk', $diskName, '--repo', $repoName)
    Assert-Equal -Run $run -Name 'gitgit cloud snapshots exits 0' -Expected 0 -Actual $snapshots.ExitCode `
        -Detail $snapshots.StdErr.Trim()
    Assert-Literal -Name 'gitgit cloud snapshots lists the snapshot the backup just took' `
        -Expected $snapshotKey -Actual $snapshots.StdOut

    $snapMatch = [regex]::Match($snapshots.StdOut, '(?m)^\s*(?<size>\d+)\s+(?<key>\S+)\s*$')
    Assert-True -Run $run -Name 'the snapshot listing carries the size and the key of one object' `
        -Condition $snapMatch.Success -Detail "got: $($snapshots.StdOut.Trim())"
    $snapshotSize = if ($snapMatch.Success) { [int]$snapMatch.Groups['size'].Value } else { 0 }
    Assert-True -Run $run -Name 'the listed snapshot is not empty' `
        -Condition ($snapshotSize -gt 0) -Detail "the listing reports $snapshotSize bytes"
    if ($snapMatch.Success) {
        Assert-Equal -Run $run -Name 'the listed key is the key the backup reported' `
            -Expected $snapshotKey -Actual $snapMatch.Groups['key'].Value
    }
    # The listing reports DISK-LOCAL names: the disk's own prefix is
    # stripped on the way out (src/cloud/mod.rs, `local_name`), so the key
    # a restore takes is the key the listing shows.
    Assert-True -Run $run -Name 'the snapshot listing reports the disk-local key, without the disk prefix' `
        -Condition (-not $snapshotKey.StartsWith($prefix)) `
        -Detail "the key '$snapshotKey' carries the disk prefix '$prefix'"

    # ══ step 9 — roll back ═════════════════════════════════════════════════
    Step 'rolling the repository back (cloud restore)'
    # A missing key must not turn this step into a no-op: the restore is
    # run with a key that cannot exist, so the step fails loudly instead of
    # quietly skipping the assertions that follow.
    $restoreKey = if ($snapshotKey) { $snapshotKey } else { "snapshots/$repoName/0000000000-deadbeef.tar" }
    $restore = Invoke-GitGit @('cloud', 'restore', $restoreKey, '--disk', $diskName, '--repo', $repoName)
    Assert-Equal -Run $run -Name 'gitgit cloud restore exits 0' -Expected 0 -Actual $restore.ExitCode `
        -Detail $restore.StdErr.Trim()
    Assert-Literal -Name 'gitgit cloud restore reports the repository and the key it restored' `
        -Expected "restored $repoName from $restoreKey" -Actual $restore.StdOut

    # The core assertions: the repository must be back the way it was.
    $headRestored = (Get-Content -Raw -LiteralPath (Join-Path $bareDir 'HEAD')).Trim()
    Assert-Equal -Run $run -Name 'the restored HEAD is the HEAD the snapshot was taken with' `
        -Expected $headBefore -Actual $headRestored
    $commitRestored = Try-Git $bareDir @('rev-parse', '--verify', "refs/heads/$branch")
    Assert-Equal -Run $run -Name 'the restored branch ref points at the original commit again' `
        -Expected $commitSha -Actual $commitRestored
    $blobRestored = Try-Git $bareDir @('cat-file', '-p', $blobSha)
    Assert-Equal -Run $run -Name 'the restored repository really holds the original blob content' `
        -Expected $blobBody -Actual $blobRestored
    Assert-True -Run $run -Name 'the marker file the damage left behind is gone after the restore' `
        -Condition (-not (Test-Path -LiteralPath $markerPath)) -Detail "expected $markerPath to be gone"
    Assert-True -Run $run -Name 'the restore left no staging directory behind' `
        -Condition (-not (Test-Path -LiteralPath (Join-Path $reposDir ".$repoName.restore-staging"))) `
        -Detail 'the staging directory the restore extracts into is still there'
    Assert-True -Run $run -Name 'the restore left no retired directory behind' `
        -Condition (-not (Test-Path -LiteralPath (Join-Path $reposDir ".$repoName.restore-retired"))) `
        -Detail 'the retired directory the restore swaps out is still there'

    # ══ step 10 — disconnect ═══════════════════════════════════════════════
    Step 'disconnecting the disk (cloud rm)'
    $rm = Invoke-GitGit @('cloud', 'rm', $diskName)
    Assert-Equal -Run $run -Name 'gitgit cloud rm exits 0' -Expected 0 -Actual $rm.ExitCode `
        -Detail $rm.StdErr.Trim()
    Assert-Literal -Name 'gitgit cloud rm reports the disk it disconnected' `
        -Expected "disconnected $diskName" -Actual $rm.StdOut
    Assert-Literal -Name 'gitgit cloud rm states that the snapshots are left alone' `
        -Expected 'the snapshots it holds are left alone' -Actual $rm.StdOut

    # The disk is gone from the registry: asserted on the registry FILE the
    # product wrote, not on the product's own report of the change.
    $registryDoc = $null
    try {
        $registryDoc = Get-Content -Raw -LiteralPath $registryPath | ConvertFrom-Json
    } catch {
        Write-Fail "the cloud disk registry is not readable JSON :: $($_.Exception.Message)"
    }
    if ($null -ne $registryDoc) {
        $registeredNames = @($registryDoc.disks.PSObject.Properties.Name)
        Assert-True -Run $run -Name 'the registry no longer lists the disconnected disk' `
            -Condition ($registeredNames -notcontains $diskName) `
            -Detail "the registry still lists: $($registeredNames -join ', ')"
    }

    # The snapshots are still on the remote: proven by reconnecting the
    # same disk and listing again. The listing is the product's own view of
    # the remote, and the size is compared with the size the first listing
    # reported, so it is the same object and not a new one.
    $reconnect = Invoke-GitGit @('cloud', 'add', $diskName,
        '--endpoint', $endpoint, '--bucket', $bucket, '--region', $region, '--prefix', $prefix)
    Assert-Equal -Run $run -Name 'the disk can be connected again after the disconnect' `
        -Expected 0 -Actual $reconnect.ExitCode -Detail $reconnect.StdErr.Trim()
    $snapshotsAfter = Invoke-GitGit @('cloud', 'snapshots', '--disk', $diskName, '--repo', $repoName)
    Assert-Equal -Run $run -Name 'gitgit cloud snapshots exits 0 after the reconnect' `
        -Expected 0 -Actual $snapshotsAfter.ExitCode -Detail $snapshotsAfter.StdErr.Trim()
    Assert-Literal -Name 'the snapshot is still on the remote after the disconnect' `
        -Expected $snapshotKey -Actual $snapshotsAfter.StdOut
    # `[FACT]` The size is looked up **by the key**, not by the first line
    # of the listing. A restore leaves a safety snapshot behind (ADR-0025
    # §2.4: the state that was replaced is kept), so the listing holds
    # more than one object by now - and "the first line" would then be
    # whichever object the store happened to list first. Comparing that
    # object's size with the backup's would be comparing two different
    # things, and it would be a false green whenever they happened to be
    # the same size.
    $snapAfterMatch = [regex]::Match(
        $snapshotsAfter.StdOut,
        '(?m)^\s*(?<size>\d+)\s+' + [regex]::Escape($snapshotKey) + '\s*$')
    Assert-Equal -Run $run -Name 'the snapshot that survived the disconnect is the same object, byte for byte in size' `
        -Expected $snapshotSize `
        -Actual $(if ($snapAfterMatch.Success) { [int]$snapAfterMatch.Groups['size'].Value } else { -1 }) `
        -Detail "the listing after the disconnect: $($snapshotsAfter.StdOut.Trim())"
    # Leave the registry in the state the check is about: disconnected.
    $rmAgain = Invoke-GitGit @('cloud', 'rm', $diskName)
    Assert-Equal -Run $run -Name 'gitgit cloud rm exits 0 the second time' `
        -Expected 0 -Actual $rmAgain.ExitCode -Detail $rmAgain.StdErr.Trim()
    $registryDoc2 = Get-Content -Raw -LiteralPath $registryPath | ConvertFrom-Json
    $registeredNames2 = @($registryDoc2.disks.PSObject.Properties.Name)
    Assert-True -Run $run -Name 'the registry is clean again at the end of the run' `
        -Condition ($registeredNames2 -notcontains $diskName) `
        -Detail "the registry still lists: $($registeredNames2 -join ', ')"

    # ══ the run itself ═════════════════════════════════════════════════════
    Assert-Equal -Run $run -Name 'every step of the check ran' -Expected 10 -Actual $script:stepCount
    # [FACT] This script makes 57 assertions when every conditional branch is
    # taken, and 53 when none of the four is. The floor below is the lower
    # of the two: a run that quietly stopped making assertions - a step that
    # was skipped, an assertion that was deleted - cannot pass it. (This is
    # the same count-based fail-closed rule scripts/verify-s3-vault.ps1
    # applies to its `cargo test` count: a filter that matches nothing exits
    # 0 and would look green.)
    #
    # [FACT] The four conditional assertions are the ones guarded by
    # `$backupMetaMatch.Success`, `$snapMatch.Success`, `$null -ne
    # $registryDoc` and `$snapAfterMatch.Success`. In a green run all four
    # fire - each is guarded by an assertion that fails when its own parse
    # fails - so a green run makes exactly 57.
    Assert-True -Run $run -Name 'the run made the number of assertions it is expected to make' `
        -Condition (($run.Pass + $run.Fail) -ge 53) `
        -Detail "the run made $($run.Pass + $run.Fail) assertions"
}
catch {
    Write-Fail "the cloud e2e check could not finish :: $($_.Exception.Message)"
    Add-RegressionCase -Run $run -Name 'the cloud e2e check ran to completion' `
        -Outcome fail -Detail $_.Exception.Message
    $exitCode = 1
}
finally {
    if ($server -and -not $server.HasExited) {
        try { $server.Kill($true) } catch { }
    }
    try {
        if ($server) {
            $server.WaitForExit(5000) | Out-Null
            @($server.StandardOutput.ReadToEnd(), $server.StandardError.ReadToEnd()) |
                Where-Object { $_ } | Set-Content -Path $serverLog -Encoding utf8
        }
    } catch { }
    if ($exitCode -eq 0 -and $run.Fail -eq 0) {
        # A green run leaves nothing behind. A red run keeps its scratch
        # tree, so the object store's data and its log can be read.
        Remove-ScratchRoot -Path $work
    } else {
        Write-Host "  the scratch tree was kept for inspection: $work" -ForegroundColor Yellow
    }
}

if ($run.Fail -gt 0 -and $exitCode -eq 0) { $exitCode = 1 }

$report = Write-RegressionReport -Run $run -OutFile (Join-Path (Get-LogRoot) 'cloud-e2e.json')

Write-Host ''
Write-Host ("  endpoint    : {0}" -f $endpoint)
Write-Host ("  bucket      : {0}" -f $bucket)
Write-Host ("  disk        : {0}" -f $diskName)
Write-Host ("  registry    : {0}" -f $registryPath)
Write-Host ("  snapshot key: {0}" -f $snapshotKey)
Write-Host ("  report      : {0}" -f (Join-Path (Get-LogRoot) 'cloud-e2e.json'))

if ($exitCode -ne 0) {
    Write-Host ''
    Write-Host ("  FAIL: {0} of {1} assertions failed" -f $run.Fail, ($run.Pass + $run.Fail)) -ForegroundColor Red
    exit 1
}

Write-Host ''
Write-Host ("  PASS: {0} assertions passed - the cloud binary connected a disk, backed up a real bare repository, rolled it back after real damage, and disconnected with the snapshot left on the remote" -f $run.Pass) -ForegroundColor Green
exit 0
