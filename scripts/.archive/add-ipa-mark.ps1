$ErrorActionPreference = 'Stop'
$dir = "D:\GitGit\docs\design\basic-design"
$files = @{
    "00-introduction.md"               = "0.7 本書構成 / 4 任务 (T4-1/2/3)"
    "01-system-overview.md"            = "P3.A1.T3 システム境界 / P3.A1.T5 システム構成要素"
    "02-architecture.md"               = "P3.A2.T1 ハードウェア・ソフトウェア構成"
    "03-functional-design.md"          = "P3.A1.T1 機能要件 / P3.A2.T2 システム機能構成"
    "04-data-design.md"                = "P3.A1.T6 データモデル / P3.A2.T3 データ構成"
    "05-interface-design.md"           = "P3.A2.T4 インタフェース構成 (総覧)"
    "06-non-functional-design.md"      = "P3.A1.T2 非機能要件 / P3.A2.T5 信頼性・性能・運用性"
    "07-security-design.md"            = "P3.A2.T6 セキュリティ方式"
    "08-operations-design.md"          = "P3.A2.T5 運用性"
    "09-migration-design.md"           = "P3.A2.T7 移行方式"
    "10-acceptance-test-policy.md"      = "P9 ソフトウェア受入プロセス 全活動"
    "11-api-design.md"                 = "P3.A2.T4 インタフェース構成 (詳細)"
    "12-app-group-intercommunication.md"= "P3.A2.T8 App 群组信息互通 (补充)"
}
foreach ($fileName in $files.Keys) {
    $path = Join-Path $dir $fileName
    $content = Get-Content $path -Raw
    $ipaMap = $files[$fileName]
    $marker = "> **[PROPOSAL] IPA 共通フレーム 2013 位置付け：** $ipaMap"

    # Check if already has the marker
    if ($content -match 'IPA 共通フレーム 2013 位置付け') {
        Write-Host "  SKIP (already has marker): $fileName"
        continue
    }

    # Insert marker after the H1 title (first line that's `# ...`)
    $lines = $content -split "`n", 0
    $newLines = @()
    $inserted = $false
    foreach ($line in $lines) {
        $newLines += $line
        if (-not $inserted -and $line -match '^# ') {
            $newLines += ""
            $newLines += $marker
            $newLines += ""
            $inserted = $true
        }
    }
    if ($inserted) {
        $newContent = $newLines -join "`n"
        [System.IO.File]::WriteAllText($path, $newContent, [System.Text.UTF8Encoding]::new($false))
        Write-Host "  OK: $fileName"
    } else {
        Write-Host "  SKIP (no H1): $fileName"
    }
}
Write-Host "`nDone."
