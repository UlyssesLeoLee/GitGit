$ErrorActionPreference = 'Stop'
$dir = "D:\GitGit\docs"
$files = Get-ChildItem $dir -File -Recurse -Filter '*.md'

# Step 1: Collect all actual section anchors from headers
$actualAnchors = @{}
foreach ($file in $files) {
    $content = Get-Content $file.FullName -Raw
    $lines = $content -split "`n"
    $relPath = $file.FullName.Substring($dir.Length + 1).Replace('\', '/')
    foreach ($line in $lines) {
        if ($line -match '^(#+)\s+(.+?)\s*$' -and $line -notmatch '^#\s*$') {
            $text = $Matches[2]
            # Compute GitHub anchor (lowercase, spaces->-, strip punct except CJK and -)
            # Also preserve ① ~ ⑳ (U+2460-U+2473, U+3251-U+325F, U+32B1-U+32BF) circled digits
            # GitHub actually keeps these in slugs (used in IPA chapter titles like "Grade ①")
            $anchor = $text.ToLower()
            $anchor = [regex]::Replace($anchor, '[\s\u3000]+', '-')
            $anchor = [regex]::Replace($anchor, '[^\w\u4e00-\u9fff\u2460-\u2473\u3251-\u325f\u32b1-\u32bf\u2776-\u2793\-]', '', 'None')
            $anchor = [regex]::Replace($anchor, '-+', '-')
            $anchor = $anchor.Trim('-')
            if (-not $actualAnchors.ContainsKey($relPath)) { $actualAnchors[$relPath] = @{} }
            $actualAnchors[$relPath][$anchor] = $text
        }
    }
}

# Step 2: Find all cross-references and verify with proper path resolution
$brokenLinks = @()
$totalLinks = 0
foreach ($file in $files) {
    $content = Get-Content $file.FullName -Raw
    $relPath = $file.FullName.Substring($dir.Length + 1).Replace('\', '/')

    $linkPattern = '\[([^\]]+)\]\(([^\)]+\.md)(#([^)]+))?\)'
    $matches = [regex]::Matches($content, $linkPattern)
    foreach ($m in $matches) {
        $totalLinks++
        $linkText = $m.Groups[1].Value
        $targetFile = $m.Groups[2].Value
        $targetAnchor = if ($m.Groups[4].Success) { $m.Groups[4].Value } else { $null }

        if ($targetFile -match '^http') { continue }

        # Resolve path: from source's dir, follow targetFile
        $sourceDir = Split-Path $relPath -Parent
        $sourceDir = $sourceDir.Replace('\', '/')
        $combined = if ($sourceDir -eq '') { $targetFile } else { "$sourceDir/$targetFile" }
        # Normalize . and ..
        $parts = $combined -split '/'
        $parts = $parts | ForEach-Object { $_.Replace('\', '/') }
        $stack = [System.Collections.Stack]::new()
        foreach ($p in $parts) {
            if ($p -eq '' -or $p -eq '.') { continue }
            if ($p -eq '..') {
                if ($stack.Count -gt 0) { [void]$stack.Pop() }
            } else {
                $stack.Push($p) | Out-Null
            }
        }
        $normalized = ($stack.ToArray() | Select-Object -Skip 1) -join '/'  # reverse order, skip the file
        # Actually rebuild in correct order
        $arr = $stack.ToArray()
        [array]::Reverse($arr)
        $normalized = $arr -join '/'

        if (-not $actualAnchors.ContainsKey($normalized)) {
            $brokenLinks += [PSCustomObject]@{ Source = $relPath; Target = $normalized; Anchor = $targetAnchor; Text = $linkText; Issue = "TARGET_FILE_MISSING" }
            continue
        }
        if ($targetAnchor -and -not $actualAnchors[$normalized].ContainsKey($targetAnchor)) {
            $brokenLinks += [PSCustomObject]@{ Source = $relPath; Target = $normalized; Anchor = $targetAnchor; Text = $linkText; Issue = "ANCHOR_MISSING" }
        }
    }
}

Write-Host "=== Anchor link audit ==="
Write-Host "  Total cross-references: $totalLinks"
Write-Host "  Broken: $($brokenLinks.Count)"
Write-Host ""
if ($brokenLinks.Count -gt 0) {
    Write-Host "=== Broken links (grouped by target) ==="
    foreach ($b in $brokenLinks) {
        Write-Host "  [$($b.Issue)] $($b.Source)"
        Write-Host "    -> $($b.Target)$(if ($b.Anchor) { '#' + $b.Anchor })"
        Write-Host "    link text: '$($b.Text)'"
    }
} else {
    Write-Host "  All cross-references resolve."
}
