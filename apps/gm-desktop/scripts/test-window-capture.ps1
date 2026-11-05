#requires -Version 7
<#
.SYNOPSIS
    Exercise the pixel half of the packaged-app launch proof with synthetic
    captures.

.DESCRIPTION
    WHY THIS TEST EXISTS
    ====================
    The `msi-launch` job captures the packaged application's window and
    asserts that it shows the product. A gate that has never been fed a
    failure is not known to be a gate, so the decision has to be shown to
    REJECT the things this lane exists to catch.

    The decision lives in `window-capture.ps1`, which has no side effects and
    needs no Windows desktop, so these cases run anywhere PowerShell runs —
    including the Linux runner of the always-on `gate` workflow, where every
    pull request is measured.

    [FACT] These cases are about the DECISION, not about the capture: the
    bytes of a capture are handed to the decision directly. The capture
    itself is exercised on Windows, in the `msi-launch` job, which captures a
    real blank window through the real capture path and asserts that the same
    decision rejects it.

    THE CASES
    =========
    Each of them is a failure the check has to catch:
      1. a flat white surface — the webview that loaded nothing;
      2. a flat surface in the product's accent colour — a splash screen;
      3. a white page with the native title bar around it — the exact thing
         a "is it one flat colour?" check is fooled by;
      4. an error screen — a white card with red text on it;
      5. the product, rendered — which has to be ACCEPTED, so the check is
         not simply refusing everything.

.NOTES
    Exit codes
        0  every case behaved as specified.
        1  at least one case did not; each failure is printed.
        2  setup error (the helper is not there, or the palette could not be
           read).
#>
[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
if (Get-Variable -Name PSNativeCommandUseErrorActionPreference -ErrorAction SilentlyContinue) {
    $PSNativeCommandUseErrorActionPreference = $false
}

$helper = Join-Path $PSScriptRoot 'window-capture.ps1'
if (-not (Test-Path -LiteralPath $helper)) {
    Write-Error "the window-capture helper '$helper' is not there"
    exit 2
}
. $helper

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..' '..' '..')).Path
$tailwindConfig = Join-Path $repoRoot 'apps/gm-desktop/tailwind.config.js'

$script:Failures = 0
$script:CaseCount = 0

function Write-Case { param([string]$M) Write-Host "    --  $M" -ForegroundColor DarkGray }
function Write-Ok   { param([string]$M) Write-Host "    ok  $M" -ForegroundColor Green }
function Write-Bad  { param([string]$M) Write-Host "    FAIL  $M" -ForegroundColor Red; $script:Failures++ }

function Assert-True {
    param([bool]$Condition, [string]$Because)
    $script:CaseCount++
    if ($Condition) { Write-Ok $Because } else { Write-Bad $Because }
}

function Assert-Equal {
    param($Expected, $Actual, [string]$Because)
    $script:CaseCount++
    if ($Expected -eq $Actual) {
        Write-Ok $Because
    } else {
        Write-Bad ("{0} (expected '{1}', got '{2}')" -f $Because, $Expected, $Actual)
    }
}

# ---------------------------------------------------------------------------
# Synthetic captures.
# ---------------------------------------------------------------------------
# [FACT] The buffers are built here rather than in `window-capture.ps1`,
# because they are a property of the TEST and not of the product.
$builder = @'
using System;

// [FACT] The buffers are in BGRA order, because that is the order GDI hands
// pixels over in and the order `Format32bppArgb` keeps in memory — the same
// order a real capture arrives in.
public static class GitGitSyntheticCapture
{
    /// <summary>A surface of one colour.</summary>
    public static byte[] Flat(int width, int height, int r, int g, int b)
    {
        byte[] px = new byte[width * height * 4];
        for (int i = 0; i < px.Length; i += 4)
        {
            px[i] = (byte)b; px[i + 1] = (byte)g; px[i + 2] = (byte)r; px[i + 3] = 255;
        }
        return px;
    }

    /// <summary>A rectangle of one colour on a surface of another.</summary>
    public static byte[] Rect(int width, int height, int br, int bg, int bb,
                              int x0, int y0, int rw, int rh, int r, int g, int b)
    {
        byte[] px = Flat(width, height, br, bg, bb);
        for (int y = y0; y < y0 + rh && y < height; y++)
        {
            for (int x = x0; x < x0 + rw && x < width; x++)
            {
                int i = (y * width + x) * 4;
                px[i] = (byte)b; px[i + 1] = (byte)g; px[i + 2] = (byte)r; px[i + 3] = 255;
            }
        }
        return px;
    }

    /// <summary>
    /// A surface that looks like a rendered product: a sidebar, a header, a
    /// content area with text-like bands, and one block in the accent
    /// colour. The text-like bands are what a real interface produces
    /// (antialiased glyphs give a lot of slightly different colours), and
    /// they are what makes the "more than one colour region" condition
    /// meaningful rather than accidental.
    /// </summary>
    public static byte[] Product(int width, int height, int accentR, int accentG, int accentB,
                                 bool withAccent)
    {
        byte[] px = Flat(width, height, 255, 255, 255);
        // the sidebar
        Fill(px, width, height, 0, 0, 160, height, 241, 245, 249);
        // the header
        Fill(px, width, height, 0, 0, width, 40, 255, 255, 255);
        // the header's bottom edge
        Fill(px, width, height, 0, 39, width, 40, 226, 232, 240);
        // the sidebar's right edge
        Fill(px, width, height, 159, 0, 160, height, 226, 232, 240);
        // text-like bands down the content area
        for (int i = 0; i < 28; i++)
        {
            int y = 60 + i * 8;
            int shade = 45 + (i * 7) % 150;
            Fill(px, width, height, 180, y, 180 + 40 + (i * 7) % 220, y + 3,
                 shade, (int)(shade * 1.15), (int)(shade * 1.45));
        }
        // the accent control
        if (withAccent)
        {
            Fill(px, width, height, 190, 340, 190 + 72, 340 + 26, accentR, accentG, accentB);
        }
        return px;
    }

    /// <summary>The native chrome around a white client area.</summary>
    public static byte[] ChromeAndWhite(int width, int height, int titleBarHeight)
    {
        byte[] px = Flat(width, height, 255, 255, 255);
        // #F3F3FF — the colour the Windows 11 title bar paints, measured.
        Fill(px, width, height, 0, 0, width, titleBarHeight, 243, 243, 255);
        return px;
    }

    static void Fill(byte[] px, int width, int height, int x0, int y0, int x1, int y1,
                     int r, int g, int b)
    {
        for (int y = Math.Max(0, y0); y < Math.Min(height, y1); y++)
        {
            for (int x = Math.Max(0, x0); x < Math.Min(width, x1); x++)
            {
                int i = (y * width + x) * 4;
                px[i] = (byte)b; px[i + 1] = (byte)g; px[i + 2] = (byte)r; px[i + 3] = 255;
            }
        }
    }
}
'@

if (-not ('GitGitSyntheticCapture' -as [type])) {
    Add-Type -TypeDefinition $builder -ErrorAction Stop
}

# ---------------------------------------------------------------------------
# The palette the check is measured against.
# ---------------------------------------------------------------------------
Write-Case "the accent palette is read out of the product's own configuration"
try {
    $accent = @(Get-ProductAccentColors -TailwindConfigPath $tailwindConfig)
} catch {
    Write-Error "the accent palette could not be read: $_"
    exit 2
}
Assert-True ($accent.Count -ge 1) 'the accent palette carries at least one chromatic colour'

$hexes = @($accent | ForEach-Object { $_.Hex.ToLowerInvariant() })
Assert-True ($hexes -contains '#22c55e') "the brand green #22c55e is one of the colours checked for"
Assert-True ($hexes -notcontains '#f0fdf4') 'the palest accent step is NOT checked for, because the native title bar matches it'
Assert-True ($hexes -notcontains '#dcfce7') 'the second-palest accent step is NOT checked for either'

$width = 640
$height = 400
$accentR = 0x22; $accentG = 0xc5; $accentB = 0x5e

function Measure-Buffer {
    param([byte[]]$Pixels, [int]$W = 640, [int]$H = 400)
    return Measure-CapturedPixels -Pixels $Pixels -Width $W -Height $H -AccentColors $accent
}

function Verdict {
    param($Measurement)
    return Test-CapturedProduct -Measurement $Measurement
}

# ---------------------------------------------------------------------------
# 1. a flat white surface
# ---------------------------------------------------------------------------
Write-Case 'a flat white surface is not the product'
$flatWhite = [GitGitSyntheticCapture]::Flat($width, $height, 255, 255, 255)
$measurement = Measure-Buffer -Pixels $flatWhite
$verdict = Verdict -Measurement $measurement
Assert-Equal 0 $measurement.AccentPixels 'the flat white surface holds no accent pixels'
Assert-Equal 1 $measurement.ColourRegions 'the flat white surface is one colour region'
Assert-True (-not $verdict.Passed) 'the flat white surface is rejected'

# ---------------------------------------------------------------------------
# 2. a flat surface in the accent colour (a splash screen)
# ---------------------------------------------------------------------------
Write-Case 'a flat surface in the accent colour is not the product either'
$flatAccent = [GitGitSyntheticCapture]::Flat($width, $height, $accentR, $accentG, $accentB)
$measurement = Measure-Buffer -Pixels $flatAccent
$verdict = Verdict -Measurement $measurement
Assert-True ($measurement.AccentPixels -gt 0) 'the flat accent surface holds accent pixels'
Assert-Equal 1 $measurement.ColourRegions 'the flat accent surface is one colour region'
Assert-True (-not $verdict.Passed) 'the flat accent surface is rejected'

# ---------------------------------------------------------------------------
# 3. a white page with the native title bar around it
# ---------------------------------------------------------------------------
Write-Case 'a white page with the native title bar is not the product'
$chrome = [GitGitSyntheticCapture]::ChromeAndWhite($width, $height, 30)
$measurement = Measure-Buffer -Pixels $chrome
$verdict = Verdict -Measurement $measurement
Assert-Equal 0 $measurement.AccentPixels 'the native title bar colour is not mistaken for the accent colour'
Assert-True (-not $verdict.Passed) 'the white page with a title bar is rejected'

# ---------------------------------------------------------------------------
# 4. an error screen: a white card with red text
# ---------------------------------------------------------------------------
Write-Case 'an error screen is not the product'
$errorScreen = [GitGitSyntheticCapture]::Rect($width, $height, 255, 255, 255, 40, 40, 400, 120, 255, 255, 255)
$errorScreen = [GitGitSyntheticCapture]::Rect($width, $height, 255, 255, 255, 60, 60, 200, 16, 220, 38, 38)
$measurement = Measure-Buffer -Pixels $errorScreen
$verdict = Verdict -Measurement $measurement
Assert-Equal 0 $measurement.AccentPixels 'the error screen holds no accent pixels'
Assert-True (-not $verdict.Passed) 'the error screen is rejected'

# ---------------------------------------------------------------------------
# 5. the product, rendered
# ---------------------------------------------------------------------------
Write-Case 'the product itself is accepted'
$product = [GitGitSyntheticCapture]::Product($width, $height, $accentR, $accentG, $accentB, $true)
$measurement = Measure-Buffer -Pixels $product
$verdict = Verdict -Measurement $measurement
Assert-True ($measurement.AccentPixels -ge 150) ("the product holds accent pixels (" + $measurement.AccentPixels + ')')
Assert-True ($measurement.ColourRegions -ge 12) ("the product holds colour regions (" + $measurement.ColourRegions + ')')
Assert-True ($verdict.Passed) 'the rendered product is accepted'

# ---------------------------------------------------------------------------
# 6. the product, rendered without the accent control
# ---------------------------------------------------------------------------
Write-Case 'a layout without the product accent is rejected'
$noAccent = [GitGitSyntheticCapture]::Product($width, $height, $accentR, $accentG, $accentB, $false)
$measurement = Measure-Buffer -Pixels $noAccent
$verdict = Verdict -Measurement $measurement
Assert-True ($measurement.ColourRegions -ge 12) 'the layout still holds colour regions'
Assert-True (-not $verdict.Passed) 'a layout without the accent colour is rejected'

Write-Host ""
if ($script:Failures -gt 0) {
    Write-Host ("{0} of {1} cases FAILED" -f $script:Failures, $script:CaseCount) -ForegroundColor Red
    exit 1
}
Write-Host ("all {0} cases passed" -f $script:CaseCount) -ForegroundColor Green
exit 0
