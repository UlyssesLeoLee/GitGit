$ErrorActionPreference = 'Stop'
$dir = "D:\GitGit\docs\design"
$allFiles = Get-ChildItem $dir -File -Recurse -Filter '*.md'
$files = $allFiles | Where-Object { $_.FullName -notmatch 'README\.md$' }  # process content files, not cover READMEs
# Actually, process all markdown files
$files = $allFiles

# Build canonical anchor map
$canonicalAnchors = @{}
foreach ($file in $files) {
    $content = Get-Content $file.FullName -Raw
    $lines = $content -split "`n"
    $relPath = $file.FullName.Substring($dir.Length + 1).Replace('\', '/')
    $canonicalAnchors[$relPath] = @{}
    foreach ($line in $lines) {
        if ($line -match '^(#+)\s+(.+?)\s*$' -and $line -notmatch '^#\s*$') {
            $text = $Matches[2]
            $anchor = $text.ToLower()
            $anchor = [regex]::Replace($anchor, '[\s\u3000]+', '-')
            $anchor = [regex]::Replace($anchor, '[^\w\u4e00-\u9fff\-]', '', 'None')
            $anchor = [regex]::Replace($anchor, '-+', '-')
            $anchor = $anchor.Trim('-')
            $canonicalAnchors[$relPath][$anchor] = $text
        }
    }
}

# Find and apply anchor fixes
$linkPattern = '\[([^\]]+)\]\(([^\)]+\.md)#([^)]+)\)'
$fixes = @{}
$missing = @{}

foreach ($file in $files) {
    $content = Get-Content $file.FullName -Raw
    $relPath = $file.FullName.Substring($dir.Length + 1).Replace('\', '/')
    $linkMatches = [regex]::Matches($content, $linkPattern)
    foreach ($m in $linkMatches) {
        $linkText = $m.Groups[1].Value
        $targetFile = $m.Groups[2].Value
        $wrongAnchor = $m.Groups[3].Value

        if ($targetFile -notmatch '\.md$') { continue }

        # Resolve target path
        $sourceDir = Split-Path $relPath -Parent
        $combined = if ($sourceDir -eq '') { $targetFile } else { "$sourceDir/$targetFile" }
        $parts = $combined -split '/'
        $stack = [System.Collections.Stack]::new()
        foreach ($p in $parts) {
            if ($p -eq '' -or $p -eq '.') { continue }
            if ($p -eq '..') {
                if ($stack.Count -gt 0) { [void]$stack.Pop() }
            } else {
                $stack.Push($p) | Out-Null
            }
        }
        $arr = $stack.ToArray()
        [array]::Reverse($arr)
        $normalized = $arr -join '/'

        if (-not $canonicalAnchors.ContainsKey($normalized)) { continue }  # file missing
        if ($canonicalAnchors[$normalized].ContainsKey($wrongAnchor)) { continue }  # anchor correct

        # Try to find a fix
        $candidate = $wrongAnchor
        $found = $false
        while ($candidate.Length -gt 0 -and $candidate -match '-+$|-[^-]+$') {
            $candidate = $candidate -replace '-+$', ''
            $candidate = $candidate -replace '-[^-]+$', ''
            if ($canonicalAnchors[$normalized].ContainsKey($candidate)) { $found = $true; break }
        }
        if (-not $found -and $wrongAnchor -match '^(\d+)-') {
            $numPrefix = $Matches[1]
            foreach ($knownAnchor in $canonicalAnchors[$normalized].Keys) {
                if ($knownAnchor.StartsWith($numPrefix + "-")) {
                    $candidate = $knownAnchor
                    $found = $true
                    break
                }
            }
        }

        if ($found) {
            $fixKey = "$relPath|$targetFile|$wrongAnchor"
            $fixes[$fixKey] = $candidate
        } else {
            $missKey = "$relPath|$targetFile|$wrongAnchor"
            $missing[$missKey] = $linkText
        }
    }
}

Write-Host "Found $($fixes.Count) anchor fixes to apply"
Write-Host "Unfixable: $($missing.Count) (will be listed)`n"

# Apply fixes
$byFile = @{}
foreach ($key in $fixes.Keys) {
    $parts = $key -split '\|'
    $sourceFile = $parts[0]
    $targetFile = $parts[1]
    $wrongAnchor = $parts[2]
    $correctAnchor = $fixes[$key]
    
    $path = Join-Path $dir $sourceFile
    $content = Get-Content $path -Raw
    $wrongRef = "$targetFile#$wrongAnchor"
    $correctRef = "$targetFile#$correctAnchor"
    $newContent = $content.Replace($wrongRef, $correctRef)
    if ($newContent -ne $content) {
        [System.IO.File]::WriteAllText($path, $newContent, [System.Text.UTF8Encoding]::new($false))
        $count = ([regex]::Matches($content, [regex]::Escape($wrongRef))).Count
        if (-not $byFile.ContainsKey($sourceFile)) { $byFile[$sourceFile] = 0 }
        $byFile[$sourceFile] += $count
    }
}

Write-Host "Applied fixes:"
foreach ($k in ($byFile.Keys | Sort-Object)) {
    Write-Host "  $k : $($byFile[$k])"
}

Write-Host "`n=== Unfixable anchors (need manual review) ==="
foreach ($key in ($missing.Keys | Sort-Object)) {
    $parts = $key -split '\|'
    Write-Host "  $($parts[0]) -> $($parts[1])#$($parts[2])  text: '$($missing[$key])'"
}
