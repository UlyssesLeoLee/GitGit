$ErrorActionPreference = 'Stop'
$dir = "D:\GitGit\docs\design\detailed-design"
$files = Get-ChildItem $dir -File -Filter '*.md'

# Fix: 'docs/design/basic-design/...' -> '../basic-design/...'
$totalFixed = 0
foreach ($file in $files) {
    $content = Get-Content $file.FullName -Raw
    $original = $content
    $new = $content -replace 'docs/design/basic-design/', '../basic-design/'
    if ($new -ne $original) {
        $count = ([regex]::Matches($original, 'docs/design/basic-design/')).Count
        [System.IO.File]::WriteAllText($file.FullName, $new, [System.Text.UTF8Encoding]::new($false))
        $totalFixed += $count
        Write-Host "  $($file.Name): fixed $count path(s)"
    }
}
Write-Host "`nTotal fixed: $totalFixed"
