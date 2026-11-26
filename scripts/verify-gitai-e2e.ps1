# End-to-end check of `gitai commit --from-diff` against a mock provider.
#
# The T7 acceptance criterion ("really talks to OpenAI") cannot be proven
# without a real credential, which this repository does not have. What this
# DOES prove is everything on our side of the socket: argument parsing, git
# diff collection, prompt construction, sanitization (tagging + redaction),
# request serialization, and response parsing. Only api.openai.com itself is
# out of scope.
#
# The mock is a synchronous HttpListener in this script while the binary
# runs as a child process. Doing it the other way round (BeginGetContext
# with a scriptblock callback) fails: the callback lands on a thread-pool
# thread that has no PowerShell runspace attached.
#
# Run: pwsh -NoProfile -File scripts/verify-gitai-e2e.ps1

$ErrorActionPreference = 'Stop'

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path

# Share the tier helpers rather than re-deriving them. Get-ScratchTempRoot in
# particular exists because $env:TEMP is a Windows-ism: on Linux PowerShell
# leaves it unset and `Join-Path $env:TEMP ...` fails with "Cannot bind
# argument to parameter 'Path' because it is null".
. (Join-Path $PSScriptRoot 'lib/regression-common.ps1')

# The binary lands wherever cargo put it, and the tier scripts deliberately
# build somewhere other than ./target: regression-st.ps1 and regression-ut.ps1
# both set CARGO_TARGET_DIR to <repo>/target-regression so they do not fight
# the main ./target directory for the cargo package-cache lock. That env var
# is process-local, so it does not reach this step -- the path has to be
# listed explicitly. Note the dash in `target-regression`.
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

# ── A throwaway repo with uncommitted changes and planted secrets ─────────
$work = Join-Path (Get-ScratchTempRoot) ('gitgit-gitai-e2e-' + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $work -Force | Out-Null

# Isolate git from the developer's global config so the fixture is the same
# on every machine.
$env:GIT_CONFIG_GLOBAL = Join-Path $work 'gitconfig'
$env:GIT_CONFIG_NOSYSTEM = '1'
$env:GIT_AUTHOR_NAME = 'e2e'; $env:GIT_AUTHOR_EMAIL = 'e2e@example.invalid'
$env:GIT_COMMITTER_NAME = 'e2e'; $env:GIT_COMMITTER_EMAIL = 'e2e@example.invalid'

& git init -q $work | Out-Null
Set-Content -Path (Join-Path $work 'app.txt') -Value 'line one' -NoNewline
& git -C $work add -A
& git -C $work commit -q -m 'seed'

# Two planted secrets. Neither may reach the provider.
$githubToken = 'ghp_0123456789abcdefghijklmnopqrstuvwxyz'
$awsKey      = 'AKIAIOSFODNN7EXAMPLE'
Add-Content -Path (Join-Path $work 'app.txt') -Value "line two $githubToken" -NoNewline
Add-Content -Path (Join-Path $work 'app.txt') -Value "line three $awsKey" -NoNewline

# ── Mock provider ─────────────────────────────────────────────────────────
$listener = [System.Net.HttpListener]::new()
$port = 0
foreach ($candidate in 38471..38520) {
    try {
        $listener.Prefixes.Clear()
        $listener.Prefixes.Add("http://127.0.0.1:$candidate/")
        $listener.Start()
        $port = $candidate
        break
    } catch { continue }
}
if ($port -eq 0) { throw 'could not bind a mock provider port' }

$stdoutFile = Join-Path $work 'stdout.txt'
$stderrFile = Join-Path $work 'stderr.txt'
# A fake credential, and the only one this script ever uses. It is named
# in a variable because the assertions below check where it does and does
# not appear: in the Authorization header, and never in the body.
$apiKey = 'sk-test-key-not-a-real-credential'
$env:GITGIT_AI_API_KEY = $apiKey

# -NoNewWindow is a Windows-only switch; on Linux Start-Process rejects it.
# Build the parameter set conditionally rather than assuming a host.
$startParams = @{
    FilePath               = $bin
    PassThru               = $true
    ArgumentList           = @('gitai', 'commit', '--repo', $work, '--from-diff',
                              '--ai-base-url', "http://127.0.0.1:$port/v1")
    RedirectStandardOutput = $stdoutFile
    RedirectStandardError  = $stderrFile
}
if (Test-IsWindows) { $startParams['NoNewWindow'] = $true }
$proc = Start-Process @startParams

# Serve exactly one request, then return.
$ctx = $listener.GetContext()
$reader = [System.IO.StreamReader]::new($ctx.Request.InputStream)
$captured = $reader.ReadToEnd()
$respBody = '{"id":"mock","model":"gpt-4o","choices":[{"message":{"role":"assistant","content":"add a second and third line"}}],"usage":{"prompt_tokens":11,"completion_tokens":5}}'
$buf = [System.Text.Encoding]::UTF8.GetBytes($respBody)
$ctx.Response.StatusCode = 200
$ctx.Response.ContentType = 'application/json'
$ctx.Response.ContentLength64 = $buf.Length
$ctx.Response.OutputStream.Write($buf, 0, $buf.Length)
$ctx.Response.Close()

# Do NOT stop the listener here. Stopping it before the child has read the
# response severs the connection and the child reports "error sending
# request" — the request was received and handled, but the answer never
# arrived. Tear down only after the child is done.
$proc.WaitForExit(30000) | Out-Null
$exit = $proc.ExitCode
$listener.Stop(); $listener.Close()
# Force a single string: Get-Content can hand back $null or an array, and
# a null `-replace` result then blows up on .Trim().
$stdout = [string]((Get-Content -Raw -ErrorAction SilentlyContinue $stdoutFile) -join '')
$stderr = [string]((Get-Content -Raw -ErrorAction SilentlyContinue $stderrFile) -join '')

function OneLine([string]$s) { (($s -replace '\r?\n', ' ').Trim()) }

Write-Host '── gitai commit --from-diff against a mock provider ──'
Write-Host ("  exit code        : {0}" -f $exit)
Write-Host ("  stdout           : {0}" -f (OneLine $stdout))
Write-Host ("  stderr           : {0}" -f (OneLine $stderr))
Write-Host ("  request captured : {0} bytes" -f $(if ($captured) { $captured.Length } else { 0 }))

if ($exit -ne 0) { Add-Failure "non-zero exit ($exit)" }
if ("$stdout" -notmatch 'add a second and third line') { Add-Failure 'assistant content not printed to stdout' }
if (-not $captured) { Add-Failure 'provider received no request' }

if ($captured) {
    if ($captured -notmatch '"model"')      { Add-Failure 'request body missing model' }
    if ($captured -notmatch '"stream":false') { Add-Failure 'request did not pin stream:false' }
    if ($captured -notmatch '<untrusted')    { Add-Failure 'untrusted content was not tagged' }

    # The redaction marker has to be COUNTED, not merely present. The
    # system prompt's own boilerplate contains the literal "[REDACTED]"
    # (src/ai/sanitize.rs::UNTRUSTED_SYSTEM_PROMPT), so a presence check
    # is satisfied even when NOT ONE secret was redacted. One occurrence
    # comes from that boilerplate and one more from each planted secret,
    # and this fixture plants exactly two — so a green run carries at
    # least three, and a run that redacted only one of the two carries
    # two and fails here.
    $markers = ([regex]::Matches($captured, [regex]::Escape('[REDACTED]'))).Count
    if ($markers -lt 3) {
        Add-Failure ("the request body carries {0} redaction marker(s); expected at least 3 (1 from the system prompt's boilerplate + 1 per planted secret, and the fixture plants 2)" -f $markers)
    }

    # The positive counterpart of the two leak checks: the diff's OWN text
    # has to reach the provider. The mock's reply is fixed, so every other
    # assertion here would still pass if the diff were empty, clamped at
    # MAX_DIFF_CHARS, or the wrong text — the model would be told nothing
    # and the gate would be green. Both halves are asserted, so a diff
    # that is only half-sent is caught too.
    foreach ($diffMarker in @('line two', 'line three')) {
        if ($captured -notmatch [regex]::Escape($diffMarker)) {
            Add-Failure "the request body does not contain '$diffMarker': the diff's own text did not reach the provider"
        }
    }

    if ($captured -match [regex]::Escape($githubToken)) {
        Add-Failure 'SECRET LEAKED: the planted GitHub token reached the provider'
    }
    if ($captured -match [regex]::Escape($awsKey)) {
        Add-Failure 'SECRET LEAKED: the planted AWS key reached the provider'
    }
    # The Authorization header is not in $captured (that is the request
    # body), so check it separately via the request object we already hold.
    # The product sends the key as a bearer token (src/ai/openai.rs:
    # `.bearer_auth(api_key)`), so the header has to carry it — and the
    # key must not appear in the body, which is what gets logged or sent
    # onward. The observed header is reported with the key masked out.
    $authHeader = $ctx.Request.Headers['Authorization']
    $seenAuth = if ($null -eq $authHeader) {
        '(no Authorization header)'
    } else {
        "$authHeader".Replace($apiKey, '<the API key>')
    }
    if ($authHeader -cne "Bearer $apiKey") {
        Add-Failure "the Authorization header is '$seenAuth'; expected 'Bearer ' followed by the GITGIT_AI_API_KEY value"
    }
    if ($captured -match [regex]::Escape($apiKey)) {
        Add-Failure 'SECRET LEAKED: the API key reached the request body'
    }
}

if ($failures.Count -gt 0) {
    Write-Host ''
    foreach ($f in $failures) { Write-Host ("  FAIL: {0}" -f $f) -ForegroundColor Red }
    Write-Host ''
    Write-Host '  captured request body:'
    Write-Host ("  {0}" -f $captured)
    exit 1
}
Write-Host '  PASS: diff collected, both secrets redacted, content tagged, response parsed' -ForegroundColor Green
exit 0
