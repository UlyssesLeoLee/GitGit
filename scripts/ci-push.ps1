# scripts/ci-push.ps1
# 推送 dev 到 origin 并触发 CI。
#
# 用法:
#   pwsh scripts/ci-push.ps1                    # 推 dev
#   pwsh scripts/ci-push.ps1 -Branch feat-x     # 推指定分支
#   pwsh scripts/ci-push.ps1 -DryRun            # 只打印不推
#
# CI 触发器在 .github/workflows/ 下, 只看 paths filter 命中才跑。
# dev 分支 push 触发 gm-console.yml + rust-backend.yml(若 paths 命中)。

[CmdletBinding()]
param(
    [string]$Branch = "dev",
    [switch]$DryRun
)

$ErrorActionPreference = "Stop"

# 永远不打印 env 内容(per 8/27 11:06 JST hard ban)
function Assert-Clean {
    param([string]$BranchName)
    $status = git status --porcelain
    if ($status) {
        Write-Host "ERROR: 工作区有 uncommitted changes:" -ForegroundColor Red
        Write-Host $status
        exit 1
    }
    $ahead = git rev-list --count "origin/$BranchName..$BranchName" 2>$null
    if (-not $ahead) {
        Write-Host "ERROR: 没有 origin/$BranchName 远程, 首次推送用:" -ForegroundColor Red
        Write-Host "  git push -u origin $BranchName"
        exit 1
    }
    Write-Host "  本地 $BranchName 比 origin/$BranchName 领先 $ahead commit"
}

function Test-CI-Paths {
    # 检查本次 push 的 commits 是否会触发 CI
    $diffRange = "origin/$Branch..$Branch"
    $touched = git diff --name-only $diffRange 2>$null
    if (-not $touched) {
        Write-Host "  (没有 commit 差异)"
        return
    }

    $gmHits = $touched | Where-Object { $_ -like 'apps/gm-console/*' -or $_ -eq 'src/server/api.rs' -or $_ -like '.github/workflows/gm-console.yml' }
    $rustHits = $touched | Where-Object { $_ -like 'src/*' -or $_ -eq 'Cargo.toml' -or $_ -eq 'Cargo.lock' -or $_ -like '.github/workflows/rust-backend.yml' }

    Write-Host ""
    Write-Host "CI 触发预估:" -ForegroundColor Cyan
    if ($gmHits) {
        Write-Host "  gm-console.yml     YES  ($($gmHits.Count) 文件)"
    } else {
        Write-Host "  gm-console.yml     no   (无 apps/gm-console/src/server/api.rs 变更)"
    }
    if ($rustHits) {
        Write-Host "  rust-backend.yml   YES  ($($rustHits.Count) 文件)"
    } else {
        Write-Host "  rust-backend.yml   no   (无 src/Cargo.toml 变更)"
    }
}

Write-Host "==> CI push helper" -ForegroundColor Cyan
Write-Host "  分支: $Branch"
Write-Host "  DryRun: $DryRun"
Write-Host ""

# 切到指定分支
$currentBranch = git branch --show-current
if ($currentBranch -ne $Branch) {
    Write-Host "  切换: $currentBranch -> $Branch"
    git checkout $Branch
}

Assert-Clean -BranchName $Branch
Test-CI-Paths

Write-Host ""
if ($DryRun) {
    Write-Host "(DryRun) git push origin $Branch"
    Write-Host "(DryRun) 完成后到 GitHub Actions tab 看运行"
    exit 0
}

Write-Host "==> 推送到 origin/$Branch ..." -ForegroundColor Cyan
git push origin $Branch
Write-Host ""
Write-Host "推送完成。" -ForegroundColor Green
Write-Host "GitHub Actions: https://github.com/$((git remote get-url origin) -replace '.*github.com[:/](.+?)/(.+?)\.git', '$1/$2')/actions"